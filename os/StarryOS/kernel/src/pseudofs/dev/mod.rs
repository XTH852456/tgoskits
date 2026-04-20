//! Special devices

#[cfg(feature = "input")]
mod event;
mod fb;
#[cfg(feature = "dev-log")]
mod log;
mod r#loop;
#[cfg(feature = "memtrack")]
mod memtrack;
mod rtc;
pub mod tty;

use alloc::{format, sync::Arc};
use core::any::Any;

use ax_errno::AxError;
use ax_sync::Mutex;
use axfs_ng_vfs::{DeviceId, Filesystem, NodeFlags, NodeType, VfsResult};
#[cfg(feature = "dev-log")]
pub use log::bind_dev_log;
use rand::{Rng, SeedableRng, rngs::SmallRng};

use crate::pseudofs::{Device, DeviceOps, DirMaker, DirMapping, SimpleDir, SimpleFs};

const RANDOM_SEED: &[u8; 32] = b"0123456789abcdef0123456789abcdef";

pub(crate) fn new_devfs() -> Filesystem {
    SimpleFs::new_with("devfs".into(), 0x01021994, builder)
}

struct Null;

impl DeviceOps for Null {
    fn read_at(&self, _buf: &mut [u8], _offset: u64) -> VfsResult<usize> {
        Ok(0)
    }

    fn write_at(&self, buf: &[u8], _offset: u64) -> VfsResult<usize> {
        Ok(buf.len())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn flags(&self) -> NodeFlags {
        NodeFlags::NON_CACHEABLE | NodeFlags::STREAM
    }
}

struct Zero;

impl DeviceOps for Zero {
    fn read_at(&self, buf: &mut [u8], _offset: u64) -> VfsResult<usize> {
        buf.fill(0);
        Ok(buf.len())
    }

    fn write_at(&self, buf: &[u8], _offset: u64) -> VfsResult<usize> {
        Ok(buf.len())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn flags(&self) -> NodeFlags {
        NodeFlags::NON_CACHEABLE | NodeFlags::STREAM
    }
}

struct Random {
    rng: Mutex<SmallRng>,
}

impl Random {
    pub fn new() -> Self {
        Self {
            rng: Mutex::new(SmallRng::from_seed(*RANDOM_SEED)),
        }
    }
}

impl DeviceOps for Random {
    fn read_at(&self, buf: &mut [u8], _offset: u64) -> VfsResult<usize> {
        self.rng.lock().fill_bytes(buf);
        Ok(buf.len())
    }

    fn write_at(&self, buf: &[u8], _offset: u64) -> VfsResult<usize> {
        Ok(buf.len())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn flags(&self) -> NodeFlags {
        NodeFlags::NON_CACHEABLE | NodeFlags::STREAM
    }
}

struct Full;

impl DeviceOps for Full {
    fn read_at(&self, buf: &mut [u8], _offset: u64) -> VfsResult<usize> {
        buf.fill(0);
        Ok(buf.len())
    }

    fn write_at(&self, _buf: &[u8], _offset: u64) -> VfsResult<usize> {
        Err(AxError::StorageFull)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn flags(&self) -> NodeFlags {
        NodeFlags::NON_CACHEABLE | NodeFlags::STREAM
    }
}

struct CpuDmaLatency;

impl DeviceOps for CpuDmaLatency {
    fn read_at(&self, _buf: &mut [u8], _offset: u64) -> VfsResult<usize> {
        Err(AxError::InvalidInput)
    }

    fn write_at(&self, buf: &[u8], _offset: u64) -> VfsResult<usize> {
        Ok(buf.len())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn flags(&self) -> NodeFlags {
        NodeFlags::NON_CACHEABLE
    }
}

fn builder(fs: Arc<SimpleFs>) -> DirMaker {
    let mut root = DirMapping::new();

    root.add(
        "null",
        Device::new(
            fs.clone(),
            NodeType::CharacterDevice,
            DeviceId::new(1, 3),
            Arc::new(Null),
        ),
    );
    super::sys::register_device(1, 3, NodeType::CharacterDevice, "null");

    root.add(
        "zero",
        Device::new(
            fs.clone(),
            NodeType::CharacterDevice,
            DeviceId::new(1, 5),
            Arc::new(Zero),
        ),
    );
    super::sys::register_device(1, 5, NodeType::CharacterDevice, "zero");

    root.add(
        "full",
        Device::new(
            fs.clone(),
            NodeType::CharacterDevice,
            DeviceId::new(1, 7),
            Arc::new(Full),
        ),
    );
    super::sys::register_device(1, 7, NodeType::CharacterDevice, "full");

    root.add(
        "random",
        Device::new(
            fs.clone(),
            NodeType::CharacterDevice,
            DeviceId::new(1, 8),
            Arc::new(Random::new()),
        ),
    );
    super::sys::register_device(1, 8, NodeType::CharacterDevice, "random");

    root.add(
        "urandom",
        Device::new(
            fs.clone(),
            NodeType::CharacterDevice,
            DeviceId::new(1, 9),
            Arc::new(Random::new()),
        ),
    );
    super::sys::register_device(1, 9, NodeType::CharacterDevice, "urandom");

    root.add(
        "rtc0",
        Device::new(
            fs.clone(),
            NodeType::CharacterDevice,
            rtc::RTC0_DEVICE_ID,
            Arc::new(rtc::Rtc),
        ),
    );
    super::sys::register_device(
        rtc::RTC0_DEVICE_ID.major(),
        rtc::RTC0_DEVICE_ID.minor(),
        NodeType::CharacterDevice,
        "rtc0",
    );

    if ax_display::has_display() {
        root.add(
            "fb0",
            Device::new(
                fs.clone(),
                NodeType::CharacterDevice,
                DeviceId::new(29, 0),
                Arc::new(fb::FrameBuffer::new()),
            ),
        );
        super::sys::register_device(29, 0, NodeType::CharacterDevice, "fb0");
    }

    root.add(
        "tty",
        Device::new(
            fs.clone(),
            NodeType::CharacterDevice,
            DeviceId::new(5, 0),
            Arc::new(tty::CurrentTty),
        ),
    );
    super::sys::register_device(5, 0, NodeType::CharacterDevice, "tty");

    root.add(
        "console",
        Device::new(
            fs.clone(),
            NodeType::CharacterDevice,
            DeviceId::new(5, 1),
            tty::N_TTY.clone(),
        ),
    );
    super::sys::register_device(5, 1, NodeType::CharacterDevice, "console");

    root.add(
        "ptmx",
        Device::new(
            fs.clone(),
            NodeType::CharacterDevice,
            DeviceId::new(5, 2),
            Arc::new(tty::Ptmx(fs.clone())),
        ),
    );
    super::sys::register_device(5, 2, NodeType::CharacterDevice, "ptmx");

    root.add(
        "pts",
        SimpleDir::new_maker(fs.clone(), Arc::new(tty::PtsDir)),
    );
    #[cfg(feature = "dev-log")]
    root.add(
        "log",
        crate::pseudofs::SimpleFile::new(fs.clone(), NodeType::Socket, || Ok(b"")),
    );

    #[cfg(feature = "memtrack")]
    root.add(
        "memtrack",
        Device::new(
            fs.clone(),
            NodeType::CharacterDevice,
            DeviceId::new(114, 514),
            Arc::new(memtrack::MemTrack),
        ),
    );
    #[cfg(feature = "memtrack")]
    super::sys::register_device(114, 514, NodeType::CharacterDevice, "memtrack");

    root.add(
        "cpu_dma_latency",
        Device::new(
            fs.clone(),
            NodeType::CharacterDevice,
            DeviceId::new(10, 1024),
            Arc::new(CpuDmaLatency),
        ),
    );
    super::sys::register_device(10, 1024, NodeType::CharacterDevice, "cpu_dma_latency");

    // This is mounted to a tmpfs in `new_procfs`
    root.add(
        "shm",
        SimpleDir::new_maker(fs.clone(), Arc::new(DirMapping::new())),
    );

    // Loop devices
    for i in 0..16 {
        let dev_id = DeviceId::new(7, i);
        root.add(
            format!("loop{i}"),
            Device::new(
                fs.clone(),
                NodeType::BlockDevice,
                dev_id,
                Arc::new(r#loop::LoopDevice::new(i, dev_id)),
            ),
        );
        super::sys::register_device(7, i, NodeType::BlockDevice, &format!("loop{i}"));
    }

    // Input devices
    #[cfg(feature = "input")]
    root.add(
        "input",
        SimpleDir::new_maker(fs.clone(), Arc::new(event::input_devices(fs.clone()))),
    );

    SimpleDir::new_maker(fs, Arc::new(root))
}
