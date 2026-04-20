//! Minimal sysfs implementation for systemd compatibility.
//!
//! Provides `/sys/dev/block/{major}:{minor}/uevent` and
//! `/sys/dev/char/{major}:{minor}/uevent` so that systemd's `chase()`
//! does not crash on an empty `/sys`.

use alloc::{
    borrow::Cow,
    boxed::Box,
    format,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};

use axfs_ng_vfs::{Filesystem, NodeType, VfsError, VfsResult};

use crate::pseudofs::{
    DirMaker, DirMapping, NodeOpsMux, SimpleDir, SimpleDirOps, SimpleFile, SimpleFs,
};

// ---------------------------------------------------------------------------
// Global device registry
// ---------------------------------------------------------------------------

struct SysDevice {
    major: u32,
    minor: u32,
    dev_type: NodeType,
    name: String,
}

static DEVICES: spin::Mutex<Vec<SysDevice>> = spin::Mutex::new(Vec::new());

/// Register a device in the sysfs device registry.
///
/// Called from devfs when each device is created.
pub fn register_device(major: u32, minor: u32, dev_type: NodeType, name: &str) {
    DEVICES.lock().push(SysDevice {
        major,
        minor,
        dev_type,
        name: name.into(),
    });
}

fn parse_major_minor(name: &str) -> VfsResult<(u32, u32)> {
    let (maj_s, min_s) = name.split_once(':').ok_or(VfsError::NotFound)?;
    let major = maj_s.parse::<u32>().map_err(|_| VfsError::NotFound)?;
    let minor = min_s.parse::<u32>().map_err(|_| VfsError::NotFound)?;
    Ok((major, minor))
}

// ---------------------------------------------------------------------------
// /sys/dev/block/{major}:{minor}/  and  /sys/dev/char/{major}:{minor}/
// ---------------------------------------------------------------------------

struct SysDevTypeDir {
    fs: Arc<SimpleFs>,
    target_type: NodeType,
}

impl SimpleDirOps for SysDevTypeDir {
    fn child_names<'a>(&'a self) -> Box<dyn Iterator<Item = Cow<'a, str>> + 'a> {
        let names: Vec<String> = DEVICES
            .lock()
            .iter()
            .filter(|d| d.dev_type == self.target_type)
            .map(|d| format!("{}:{}", d.major, d.minor))
            .collect();
        Box::new(names.into_iter().map(|s| s.into()))
    }

    fn lookup_child(&self, name: &str) -> VfsResult<NodeOpsMux> {
        let (major, minor) = parse_major_minor(name)?;
        let (dev_name, dev_type) = DEVICES
            .lock()
            .iter()
            .find(|d| d.major == major && d.minor == minor && d.dev_type == self.target_type)
            .map(|d| (d.name.clone(), d.dev_type))
            .ok_or(VfsError::NotFound)?;

        Ok(NodeOpsMux::Dir(SimpleDir::new_maker(
            self.fs.clone(),
            Arc::new(SysDevEntryDir {
                major,
                minor,
                dev_type,
                dev_name,
                fs: self.fs.clone(),
            }),
        )))
    }

    fn is_cacheable(&self) -> bool {
        false
    }
}

/// A single `/sys/dev/block/{major}:{minor}/` directory containing `uevent`.
struct SysDevEntryDir {
    major: u32,
    minor: u32,
    dev_type: NodeType,
    dev_name: String,
    fs: Arc<SimpleFs>,
}

impl SimpleDirOps for SysDevEntryDir {
    fn child_names<'a>(&'a self) -> Box<dyn Iterator<Item = Cow<'a, str>> + 'a> {
        Box::new(Some(Cow::Borrowed("uevent")).into_iter())
    }

    fn lookup_child(&self, name: &str) -> VfsResult<NodeOpsMux> {
        match name {
            "uevent" => {
                let content = self.uevent_content();
                Ok(SimpleFile::new_regular(self.fs.clone(), move || Ok(content.clone())).into())
            }
            _ => Err(VfsError::NotFound),
        }
    }

    fn is_cacheable(&self) -> bool {
        false
    }
}

impl SysDevEntryDir {
    fn uevent_content(&self) -> String {
        let devtype = match self.dev_type {
            NodeType::BlockDevice => "disk",
            _ => "",
        };
        if devtype.is_empty() {
            format!("MAJOR={}\nMINOR={}\nDEVNAME={}\n", self.major, self.minor, self.dev_name)
        } else {
            format!(
                "MAJOR={}\nMINOR={}\nDEVNAME={}\nDEVTYPE={}\n",
                self.major, self.minor, self.dev_name, devtype
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Sysfs builder
// ---------------------------------------------------------------------------

fn builder(fs: Arc<SimpleFs>) -> DirMaker {
    let mut root = DirMapping::new();

    // /sys/dev/block/
    root.add(
        "dev",
        {
            let mut dev = DirMapping::new();
            dev.add(
                "block",
                SimpleDir::new_maker(
                    fs.clone(),
                    Arc::new(SysDevTypeDir {
                        fs: fs.clone(),
                        target_type: NodeType::BlockDevice,
                    }),
                ),
            );
            dev.add(
                "char",
                SimpleDir::new_maker(
                    fs.clone(),
                    Arc::new(SysDevTypeDir {
                        fs: fs.clone(),
                        target_type: NodeType::CharacterDevice,
                    }),
                ),
            );
            SimpleDir::new_maker(fs.clone(), Arc::new(dev))
        },
    );

    // /sys/class/graphics/fb0/device/subsystem  (symlink chain for display)
    root.add(
        "class",
        {
            let mut class = DirMapping::new();
            class.add(
                "graphics",
                {
                    let mut graphics = DirMapping::new();
                    graphics.add(
                        "fb0",
                        {
                            let mut fb0 = DirMapping::new();
                            fb0.add(
                                "device",
                                {
                                    let mut device = DirMapping::new();
                                    device.add(
                                        "subsystem",
                                        SimpleFile::new(fs.clone(), NodeType::Symlink, || {
                                            Ok("../../../../../../../class/graphics")
                                        }),
                                    );
                                    SimpleDir::new_maker(fs.clone(), Arc::new(device))
                                },
                            );
                            SimpleDir::new_maker(fs.clone(), Arc::new(fb0))
                        },
                    );
                    SimpleDir::new_maker(fs.clone(), Arc::new(graphics))
                },
            );
            SimpleDir::new_maker(fs.clone(), Arc::new(class))
        },
    );

    // /sys/block/  — symlinks to /sys/dev/block/{major}:{minor}
    root.add(
        "block",
        SimpleDir::new_maker(
            fs.clone(),
            Arc::new(SysBlockSymlinkDir { fs: fs.clone() }),
        ),
    );

    // /sys/devices/system/cpu/online
    root.add(
        "devices",
        {
            let mut devices = DirMapping::new();
            devices.add(
                "system",
                {
                    let mut system = DirMapping::new();
                    system.add(
                        "cpu",
                        {
                            let mut cpu = DirMapping::new();
                            cpu.add(
                                "online",
                                SimpleFile::new_regular(fs.clone(), || Ok("0\n")),
                            );
                            SimpleDir::new_maker(fs.clone(), Arc::new(cpu))
                        },
                    );
                    SimpleDir::new_maker(fs.clone(), Arc::new(system))
                },
            );
            SimpleDir::new_maker(fs.clone(), Arc::new(devices))
        },
    );

    // /sys/fs/cgroup/  (empty dir, needed by systemd)
    root.add(
        "fs",
        {
            let mut fs_dir = DirMapping::new();
            fs_dir.add(
                "cgroup",
                SimpleDir::new_maker(fs.clone(), Arc::new(DirMapping::new())),
            );
            SimpleDir::new_maker(fs.clone(), Arc::new(fs_dir))
        },
    );

    // /sys/subsystem/  (empty dir)
    root.add(
        "subsystem",
        SimpleDir::new_maker(fs.clone(), Arc::new(DirMapping::new())),
    );

    SimpleDir::new_maker(fs, Arc::new(root))
}

// ---------------------------------------------------------------------------
// /sys/block/ — symlinks like loop0 -> ../dev/block/7:0
// ---------------------------------------------------------------------------

struct SysBlockSymlinkDir {
    fs: Arc<SimpleFs>,
}

impl SimpleDirOps for SysBlockSymlinkDir {
    fn child_names<'a>(&'a self) -> Box<dyn Iterator<Item = Cow<'a, str>> + 'a> {
        let names: Vec<String> = DEVICES
            .lock()
            .iter()
            .filter(|d| d.dev_type == NodeType::BlockDevice)
            .map(|d| d.name.clone())
            .collect();
        Box::new(names.into_iter().map(|s| s.into()))
    }

    fn lookup_child(&self, name: &str) -> VfsResult<NodeOpsMux> {
        let (major, minor) = DEVICES
            .lock()
            .iter()
            .find(|d| d.dev_type == NodeType::BlockDevice && d.name == name)
            .map(|d| (d.major, d.minor))
            .ok_or(VfsError::NotFound)?;
        let target = format!("../dev/block/{}:{}", major, minor);
        Ok(SimpleFile::new(self.fs.clone(), NodeType::Symlink, move || Ok(target.clone())).into())
    }

    fn is_cacheable(&self) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Create a new sysfs filesystem instance.
pub fn new_sysfs() -> Filesystem {
    SimpleFs::new_with("sysfs".into(), 0x6265, builder)
}
