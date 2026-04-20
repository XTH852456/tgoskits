//! Minimal cgroup2 filesystem for systemd compatibility.
//!
//! Provides a filesystem where each directory automatically contains
//! virtual cgroup files (cgroup.procs, cgroup.controllers, etc.).
//! systemd needs these files to create and manage cgroup scopes.

use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::{
    any::Any,
    task::Context,
    time::Duration,
};

use axfs_ng_vfs::{
    DirEntry, DirEntrySink, DirNode, DirNodeOps, FileNode, FileNodeOps, Filesystem,
    FilesystemOps, Metadata, MetadataUpdate, NodeFlags, NodeOps, NodePermission, NodeType,
    Reference, StatFs, VfsError, VfsResult, WeakDirEntry,
};
use ax_sync::Mutex;
use axpoll::{IoEvents, Pollable};
use hashbrown::HashMap;

use crate::pseudofs::dummy_stat_fs;

// ---------------------------------------------------------------------------
// Cgroup virtual file definitions
// ---------------------------------------------------------------------------

/// (filename, default_content, writable)
const CGROUP_FILES: &[(&str, &str, bool)] = &[
    ("cgroup.procs", "", true),
    ("cgroup.controllers", "", false),
    ("cgroup.subtree_control", "", true),
    ("cgroup.events", "populated 0\nfrozen 0\n", false),
    ("cgroup.max.depth", "max\n", true),
    ("cgroup.max.descendants", "max\n", true),
    ("cgroup.type", "normal\n", true),
    ("cgroup.freeze", "0\n", true),
];

// ---------------------------------------------------------------------------
// CgroupFile — virtual cgroup file node
// ---------------------------------------------------------------------------

struct CgroupFile {
    fs: Arc<CgroupFsInner>,
    content: Mutex<String>,
    writable: bool,
}

impl CgroupFile {
    fn new(fs: Arc<CgroupFsInner>, name: &str) -> Self {
        let (content, writable) = CGROUP_FILES
            .iter()
            .find(|(n, _, _)| *n == name)
            .map(|(_, c, w)| (c.to_string(), *w))
            .unwrap_or((String::new(), false));
        Self {
            fs,
            content: Mutex::new(content),
            writable,
        }
    }
}

impl FileNodeOps for CgroupFile {
    fn read_at(&self, buf: &mut [u8], offset: u64) -> VfsResult<usize> {
        let content = self.content.lock();
        let data = content.as_bytes();
        let start = offset as usize;
        if start >= data.len() {
            return Ok(0);
        }
        let len = buf.len().min(data.len() - start);
        buf[..len].copy_from_slice(&data[start..start + len]);
        Ok(len)
    }

    fn write_at(&self, buf: &[u8], _offset: u64) -> VfsResult<usize> {
        if !self.writable {
            return Err(VfsError::PermissionDenied);
        }
        Ok(buf.len())
    }

    fn append(&self, buf: &[u8]) -> VfsResult<(usize, u64)> {
        if !self.writable {
            return Err(VfsError::PermissionDenied);
        }
        Ok((buf.len(), 0))
    }

    fn set_len(&self, _len: u64) -> VfsResult<()> {
        Err(VfsError::PermissionDenied)
    }

    fn set_symlink(&self, _target: &str) -> VfsResult<()> {
        Err(VfsError::PermissionDenied)
    }
}

impl NodeOps for CgroupFile {
    fn inode(&self) -> u64 {
        self as *const _ as u64
    }

    fn metadata(&self) -> VfsResult<Metadata> {
        let content = self.content.lock();
        Ok(Metadata {
            device: 0,
            inode: self.inode(),
            nlink: 1,
            mode: if self.writable {
                NodePermission::from_bits_truncate(0o644)
            } else {
                NodePermission::from_bits_truncate(0o444)
            },
            node_type: NodeType::RegularFile,
            uid: 0,
            gid: 0,
            size: content.len() as u64,
            block_size: 4096,
            blocks: 0,
            rdev: axfs_ng_vfs::DeviceId::default(),
            atime: Duration::default(),
            mtime: Duration::default(),
            ctime: Duration::default(),
        })
    }

    fn update_metadata(&self, _update: MetadataUpdate) -> VfsResult<()> {
        Ok(())
    }

    fn filesystem(&self) -> &dyn FilesystemOps {
        self.fs.as_ref()
    }

    fn sync(&self, _data_only: bool) -> VfsResult<()> {
        Ok(())
    }

    fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }

    fn flags(&self) -> NodeFlags {
        NodeFlags::NON_CACHEABLE
    }
}

impl Pollable for CgroupFile {
    fn poll(&self) -> IoEvents {
        IoEvents::IN | IoEvents::OUT
    }

    fn register(&self, _context: &mut Context<'_>, _events: IoEvents) {}
}

// ---------------------------------------------------------------------------
// CgroupFsInner — filesystem state
// ---------------------------------------------------------------------------

struct CgroupFsInner {
    root: Mutex<Option<DirEntry>>,
}

impl CgroupFsInner {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            root: Mutex::new(None),
        })
    }
}

impl FilesystemOps for CgroupFsInner {
    fn name(&self) -> &str {
        "cgroup2"
    }

    fn root_dir(&self) -> DirEntry {
        self.root.lock().clone().unwrap()
    }

    fn stat(&self) -> VfsResult<StatFs> {
        Ok(dummy_stat_fs(0x63677270)) // CGROUP2_SUPER_MAGIC
    }
}

// ---------------------------------------------------------------------------
// CgroupDirState — shared state for a single cgroup directory
// ---------------------------------------------------------------------------

struct CgroupDirState {
    fs: Arc<CgroupFsInner>,
    children: Mutex<HashMap<String, Arc<CgroupDirState>>>,
}

impl CgroupDirState {
    fn new(fs: Arc<CgroupFsInner>) -> Arc<Self> {
        Arc::new(Self {
            fs,
            children: Mutex::new(HashMap::new()),
        })
    }
}

// ---------------------------------------------------------------------------
// CgroupDirNode — VFS node for a cgroup directory
// ---------------------------------------------------------------------------

struct CgroupDirNode {
    fs: Arc<CgroupFsInner>,
    state: Arc<CgroupDirState>,
    this: Option<WeakDirEntry>,
}

impl CgroupDirNode {
    fn new(
        fs: Arc<CgroupFsInner>,
        state: Arc<CgroupDirState>,
        this: Option<WeakDirEntry>,
    ) -> Arc<Self> {
        Arc::new(Self { fs, state, this })
    }

    fn make_dir_entry(&self, name: &str, child_state: Arc<CgroupDirState>) -> DirEntry {
        let parent = self.this.as_ref().and_then(WeakDirEntry::upgrade);
        let reference = Reference::new(parent, name.to_string());
        let fs = self.fs.clone();
        DirEntry::new_dir(
            |this| DirNode::new(CgroupDirNode::new(fs, child_state, Some(this))),
            reference,
        )
    }

    fn make_file_entry(&self, name: &str) -> DirEntry {
        let parent = self.this.as_ref().and_then(WeakDirEntry::upgrade);
        let reference = Reference::new(parent, name.to_string());
        let file_ops = Arc::new(CgroupFile::new(self.fs.clone(), name));
        DirEntry::new_file(FileNode::new(file_ops), NodeType::RegularFile, reference)
    }

    fn next_inode(&self) -> u64 {
        // Use the state pointer as a unique inode number
        Arc::as_ptr(&self.state) as u64
    }
}

impl NodeOps for CgroupDirNode {
    fn inode(&self) -> u64 {
        self.next_inode()
    }

    fn metadata(&self) -> VfsResult<Metadata> {
        let child_count = self.state.children.lock().len();
        Ok(Metadata {
            device: 0,
            inode: self.inode(),
            nlink: 2,
            mode: NodePermission::from_bits_truncate(0o755),
            node_type: NodeType::Directory,
            uid: 0,
            gid: 0,
            size: (child_count + CGROUP_FILES.len() + 2) as u64,
            block_size: 4096,
            blocks: 0,
            rdev: axfs_ng_vfs::DeviceId::default(),
            atime: Duration::default(),
            mtime: Duration::default(),
            ctime: Duration::default(),
        })
    }

    fn update_metadata(&self, _update: MetadataUpdate) -> VfsResult<()> {
        Ok(())
    }

    fn filesystem(&self) -> &dyn FilesystemOps {
        self.fs.as_ref()
    }

    fn sync(&self, _data_only: bool) -> VfsResult<()> {
        Ok(())
    }

    fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }

    fn flags(&self) -> NodeFlags {
        NodeFlags::empty()
    }
}

impl Pollable for CgroupDirNode {
    fn poll(&self) -> IoEvents {
        IoEvents::IN | IoEvents::OUT
    }

    fn register(&self, _context: &mut Context<'_>, _events: IoEvents) {}
}

impl DirNodeOps for CgroupDirNode {
    fn read_dir(&self, offset: u64, sink: &mut dyn DirEntrySink) -> VfsResult<usize> {
        let children = self.state.children.lock();
        let mut count = 0;
        let mut idx = 0u64;

        // List child directories
        for name in children.keys() {
            if idx >= offset {
                if !sink.accept(name, idx + 1, NodeType::Directory, idx + 1) {
                    return Ok(count);
                }
                count += 1;
            }
            idx += 1;
        }

        // List virtual cgroup files
        for (name, _, _) in CGROUP_FILES {
            if idx >= offset {
                if !sink.accept(name, idx + 1, NodeType::RegularFile, idx + 1) {
                    return Ok(count);
                }
                count += 1;
            }
            idx += 1;
        }

        Ok(count)
    }

    fn lookup(&self, name: &str) -> VfsResult<DirEntry> {
        // Check virtual cgroup files first
        if CGROUP_FILES.iter().any(|(n, _, _)| *n == name) {
            return Ok(self.make_file_entry(name));
        }

        // Check child directories
        let children = self.state.children.lock();
        if let Some(child) = children.get(name) {
            return Ok(self.make_dir_entry(name, child.clone()));
        }

        Err(VfsError::NotFound)
    }

    fn is_cacheable(&self) -> bool {
        false
    }

    fn create(
        &self,
        name: &str,
        node_type: NodeType,
        _permission: NodePermission,
    ) -> VfsResult<DirEntry> {
        if node_type != NodeType::Directory {
            return Err(VfsError::OperationNotPermitted);
        }

        let mut children = self.state.children.lock();
        if children.contains_key(name) {
            return Err(VfsError::AlreadyExists);
        }

        let child_state = CgroupDirState::new(self.fs.clone());
        children.insert(name.to_string(), child_state.clone());
        Ok(self.make_dir_entry(name, child_state))
    }

    fn link(&self, _name: &str, _node: &DirEntry) -> VfsResult<DirEntry> {
        Err(VfsError::OperationNotPermitted)
    }

    fn unlink(&self, name: &str) -> VfsResult<()> {
        let mut children = self.state.children.lock();
        if children.remove(name).is_some() {
            Ok(())
        } else if CGROUP_FILES.iter().any(|(n, _, _)| *n == name) {
            Err(VfsError::OperationNotPermitted)
        } else {
            Err(VfsError::NotFound)
        }
    }

    fn rename(&self, _src_name: &str, _dst_dir: &DirNode, _dst_name: &str) -> VfsResult<()> {
        Err(VfsError::OperationNotPermitted)
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Create a new cgroup2 filesystem instance.
pub fn new_cgroup2_fs() -> Filesystem {
    let inner = CgroupFsInner::new();
    let root_state = CgroupDirState::new(inner.clone());

    let root_entry = DirEntry::new_dir(
        |this| {
            let fs = inner.clone();
            let state = root_state.clone();
            DirNode::new(CgroupDirNode::new(fs, state, Some(this)))
        },
        Reference::root(),
    );

    *inner.root.lock() = Some(root_entry);
    Filesystem::new(inner)
}
