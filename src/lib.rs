use std::any::Any;
use std::mem;

use memflow::prelude::v1::*;

use memflow_vdm::{PhysicalMemory, Result, *};

use windows::core::s;
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE};
use windows::Win32::Storage::FileSystem::{CreateFileA, FILE_ATTRIBUTE_NORMAL, OPEN_EXISTING};
use windows::Win32::System::IO::DeviceIoControl;

#[repr(u32)]
enum IoControlCode {
    MapPhysicalMemory = 0x80102040,
    UnmapPhysicalMemory = 0x80102044,
}

#[derive(Default)]
#[repr(C)]
struct PhysicalMemoryMappingRequest {
    size: u64,
    phys_addr: u64,
    section_handle: HANDLE,
    virt_addr: u64,
    obj_handle: HANDLE,
}

#[derive(Clone)]
struct WinIoDriver {
    handle: HANDLE,
}

impl WinIoDriver {
    fn open() -> Result<Self> {
        let handle = unsafe {
            CreateFileA(
                s!(r"\\.\WinIo"),
                GENERIC_READ.0 | GENERIC_WRITE.0,
                Default::default(),
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )?
        };

        Ok(Self { handle })
    }
}

impl Drop for WinIoDriver {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            unsafe { _ = CloseHandle(self.handle) }
        }
    }
}

struct MapPhysicalMemoryResponse {
    phys_addr: u64,
    obj_handle: HANDLE,
    section_handle: HANDLE,
    size: usize,
    virt_addr: u64,
}

impl PhysicalMemoryResponse for MapPhysicalMemoryResponse {
    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn phys_addr(&self) -> u64 {
        self.phys_addr
    }

    #[inline]
    fn size(&self) -> usize {
        self.size
    }

    #[inline]
    fn virt_addr(&self) -> u64 {
        self.virt_addr
    }
}

impl PhysicalMemory for WinIoDriver {
    fn map_phys_mem(&self, addr: u64, size: usize) -> Result<PhysicalMemoryResponseBoxed> {
        let mut req = PhysicalMemoryMappingRequest {
            size: size as _,
            phys_addr: addr,
            ..Default::default()
        };

        unsafe {
            DeviceIoControl(
                self.handle,
                IoControlCode::MapPhysicalMemory as _,
                Some(&req as *const _ as *const _),
                mem::size_of::<PhysicalMemoryMappingRequest>() as _,
                Some(&mut req as *mut _ as *mut _),
                mem::size_of::<PhysicalMemoryMappingRequest>() as _,
                None,
                None,
            )?;
        }

        Ok(Box::new(MapPhysicalMemoryResponse {
            phys_addr: addr,
            obj_handle: req.obj_handle,
            section_handle: req.section_handle,
            size,
            virt_addr: req.virt_addr,
        }))
    }

    fn unmap_phys_mem(&self, mapping: PhysicalMemoryResponseBoxed) -> Result<()> {
        let res = mapping
            .as_any()
            .downcast_ref::<MapPhysicalMemoryResponse>()
            .unwrap();

        let req = PhysicalMemoryMappingRequest {
            obj_handle: res.obj_handle,
            section_handle: res.section_handle,
            virt_addr: res.virt_addr(),
            ..Default::default()
        };

        unsafe {
            DeviceIoControl(
                self.handle,
                IoControlCode::UnmapPhysicalMemory as _,
                Some(&req as *const _ as *const _),
                mem::size_of::<PhysicalMemoryMappingRequest>() as _,
                None,
                0,
                None,
                None,
            )
            .map_err(memflow_vdm::Error::Windows)
        }
    }
}

#[connector(name = "winio")]
pub fn create_connector<'a>(_args: &ConnectorArgs) -> memflow::error::Result<VdmConnector<'a>> {
    let driver = WinIoDriver::open().map_err(|_| {
        Error(ErrorOrigin::Connector, ErrorKind::Uninitialized)
            .log_error("Unable to open a handle to the WinIo driver")
    })?;

    init_connector(Box::new(driver)).map_err(|_| {
        Error(ErrorOrigin::Connector, ErrorKind::Uninitialized)
            .log_error("Unable to initialize the VDM connector")
    })
}
