use std::fs::{File, OpenOptions};
use std::io::{Error, ErrorKind};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};

const VFIO_GET_API_VERSION: u64 = 15204;
const VFIO_CHECK_EXTENSION: u64 = 15205;
const VFIO_GROUP_GET_STATUS: u64 = 15207;
const VFIO_GROUP_SET_CONTAINER: u64 = 15208;
const VFIO_GROUP_GET_DEVICE_FD: u64 = 15210;
const VFIO_DEVICE_GET_REGION_INFO: u64 = 15212;
const VFIO_DEVICE_SET_IRQS: u64 = 15214;

#[repr(C)]
pub struct VfioIrqSet {
    pub argsz: u32,
    pub flags: u32,
    pub index: u32,
    pub start: u32,
    pub count: u32,
    pub data: i32,
}

const VFIO_IRQ_SET_DATA_NONE: u32 = 1 << 0;
const VFIO_IRQ_SET_DATA_BOOL: u32 = 1 << 1;
const VFIO_IRQ_SET_DATA_EVENTFD: u32 = 1 << 2;
const VFIO_IRQ_SET_ACTION_MASK: u32 = 1 << 3;
const VFIO_IRQ_SET_ACTION_UNMASK: u32 = 1 << 4;
const VFIO_IRQ_SET_ACTION_TRIGGER: u32 = 1 << 5;

const VFIO_API_VERSION: i32 = 0;
const VFIO_TYPE1_IOMMU: i32 = 1;

#[repr(C)]
pub struct VfioGroupStatus {
    pub argsz: u32,
    pub flags: u32,
}
const VFIO_GROUP_FLAGS_VIABLE: u32 = 1;

#[repr(C)]
pub struct VfioDeviceRegionInfo {
    pub argsz: u32,
    pub flags: u32,
    pub index: u32,
    pub cap_offset: u32,
    pub size: u64,
    pub offset: u64,
}

pub struct VfioContainer {
    fd: File,
}

impl VfioContainer {
    pub fn new() -> Result<Self, Error> {
        let fd = OpenOptions::new().read(true).write(true).open("/dev/vfio/vfio")?;
        unsafe {
            let version = libc::ioctl(fd.as_raw_fd(), VFIO_GET_API_VERSION);
            if version != VFIO_API_VERSION {
                return Err(Error::new(ErrorKind::Other, "Unknown VFIO API version"));
            }
            let ext = libc::ioctl(fd.as_raw_fd(), VFIO_CHECK_EXTENSION, VFIO_TYPE1_IOMMU);
            if ext != 1 {
                return Err(Error::new(ErrorKind::Other, "VFIO_TYPE1_IOMMU not supported"));
            }
        }
        Ok(Self { fd })
    }
    pub fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

pub struct VfioGroup {
    fd: File,
}

impl VfioGroup {
    pub fn new(group_id: u32) -> Result<Self, Error> {
        let path = format!("/dev/vfio/{}", group_id);
        let fd = OpenOptions::new().read(true).write(true).open(&path)?;
        let mut status =
            VfioGroupStatus { argsz: std::mem::size_of::<VfioGroupStatus>() as u32, flags: 0 };
        unsafe {
            if libc::ioctl(fd.as_raw_fd(), VFIO_GROUP_GET_STATUS, &mut status as *mut _) < 0 {
                return Err(Error::last_os_error());
            }
        }
        if (status.flags & VFIO_GROUP_FLAGS_VIABLE) == 0 {
            return Err(Error::new(ErrorKind::Other, "VFIO group not viable"));
        }
        Ok(Self { fd })
    }

    pub fn set_container(&self, container: &VfioContainer) -> Result<(), Error> {
        unsafe {
            let container_fd = container.as_raw_fd();
            let ret = libc::ioctl(
                self.fd.as_raw_fd(),
                VFIO_GROUP_SET_CONTAINER,
                &container_fd as *const _,
            );
            if ret < 0 {
                return Err(Error::last_os_error());
            }
        }
        Ok(())
    }

    pub fn get_device(&self, name: &str) -> std::io::Result<VfioDevice> {
        unsafe {
            let cname = std::ffi::CString::new(name).unwrap();
            let device_fd =
                libc::ioctl(self.fd.as_raw_fd(), VFIO_GROUP_GET_DEVICE_FD, cname.as_ptr());
            if device_fd < 0 {
                return Err(Error::last_os_error());
            }
            Ok(VfioDevice { fd: File::from_raw_fd(device_fd) })
        }
    }
}

pub struct VfioDevice {
    fd: File,
}

impl VfioDevice {
    pub fn map_region(&self, index: u32) -> std::io::Result<(usize, usize)> {
        let mut info = VfioDeviceRegionInfo {
            argsz: std::mem::size_of::<VfioDeviceRegionInfo>() as u32,
            flags: 0,
            index,
            cap_offset: 0,
            size: 0,
            offset: 0,
        };
        unsafe {
            if libc::ioctl(self.fd.as_raw_fd(), VFIO_DEVICE_GET_REGION_INFO, &mut info as *mut _)
                < 0
            {
                return Err(Error::last_os_error());
            }
            let ptr = libc::mmap(
                std::ptr::null_mut(),
                info.size as libc::size_t,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                self.fd.as_raw_fd(),
                info.offset as libc::off_t,
            );
            if ptr == libc::MAP_FAILED {
                return Err(Error::last_os_error());
            }
            Ok((ptr as usize, info.size as usize))
        }
    }

    pub fn enable_irq(&self, index: u32, eventfd: i32) -> std::io::Result<()> {
        let mut irq_set = VfioIrqSet {
            argsz: std::mem::size_of::<VfioIrqSet>() as u32,
            flags: VFIO_IRQ_SET_DATA_EVENTFD | VFIO_IRQ_SET_ACTION_TRIGGER,
            index,
            start: 0,
            count: 1,
            data: eventfd,
        };
        unsafe {
            if libc::ioctl(self.fd.as_raw_fd(), VFIO_DEVICE_SET_IRQS, &mut irq_set as *mut _) < 0 {
                return Err(Error::last_os_error());
            }
        }
        Ok(())
    }

    pub fn unmask_irq(&self, index: u32) -> std::io::Result<()> {
        let mut irq_set = VfioIrqSet {
            argsz: std::mem::size_of::<VfioIrqSet>() as u32,
            flags: VFIO_IRQ_SET_DATA_NONE | VFIO_IRQ_SET_ACTION_UNMASK,
            index,
            start: 0,
            count: 1,
            data: 0,
        };
        unsafe {
            if libc::ioctl(self.fd.as_raw_fd(), VFIO_DEVICE_SET_IRQS, &mut irq_set as *mut _) < 0 {
                return Err(Error::last_os_error());
            }
        }
        Ok(())
    }
}

impl Clone for VfioGroup {
    fn clone(&self) -> Self {
        Self { fd: self.fd.try_clone().unwrap() }
    }
}

impl std::fmt::Debug for VfioGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VfioGroup(fd: {})", self.fd.as_raw_fd())
    }
}

impl Clone for VfioDevice {
    fn clone(&self) -> Self {
        Self { fd: self.fd.try_clone().unwrap() }
    }
}

impl std::fmt::Debug for VfioDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VfioDevice(fd: {})", self.fd.as_raw_fd())
    }
}
