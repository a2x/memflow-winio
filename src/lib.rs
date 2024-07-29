use std::any::Any;
use std::mem;

use memflow::prelude::v1::*;
use memflow_vdm::{PhysicalMemory, *};

use windows::core::{s, Result};
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE};
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::IO::DeviceIoControl;

use handle::RawHandle;

mod handle;

#[repr(u32)]
enum IoControlCode {
    MapPhysicalMemory = 0x80102040,
    UnmapPhysicalMemory = 0x80102044,
}

#[derive(Debug, Default)]
#[repr(C)]
struct PhysicalMemoryMappingRequest {
    size: u64,
    phys_addr: u64,
    section_handle: RawHandle,
    virt_addr: u64,
    obj_handle: RawHandle,
}

#[derive(Clone)]
struct WinIoDriver {
    handle: RawHandle,
}

impl WinIoDriver {
    fn open() -> Result<Self> {
        let handle = unsafe {
            CreateFileA(
                s!(r"\\.\WinIo"),
                GENERIC_READ.0 | GENERIC_WRITE.0,
                FILE_SHARE_MODE(0),
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )?
        };

        Ok(Self {
            handle: handle.into(),
        })
    }
}

impl Drop for WinIoDriver {
    fn drop(&mut self) {
        if self.handle.is_valid() {
            unsafe {
                let _ = CloseHandle(self.handle.handle());
            }
        }
    }
}

#[derive(Debug)]
struct MapPhysicalMemoryResponse {
    phys_addr: u64,
    obj_handle: RawHandle,
    section_handle: RawHandle,
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
    fn map_phys_mem(
        &self,
        addr: u64,
        size: usize,
    ) -> memflow_vdm::Result<PhysicalMemoryResponseBoxed> {
        let mut req = PhysicalMemoryMappingRequest {
            size: size as _,
            phys_addr: addr,
            ..Default::default()
        };

        unsafe {
            DeviceIoControl(
                self.handle.handle(),
                IoControlCode::MapPhysicalMemory as _,
                Some(&req as *const _ as *const _),
                mem::size_of::<PhysicalMemoryMappingRequest>() as _,
                Some(&mut req as *mut _ as *mut _),
                mem::size_of::<PhysicalMemoryMappingRequest>() as _,
                None,
                None,
            )
            .map_err(memflow_vdm::Error::Windows)?;
        }

        Ok(Box::new(MapPhysicalMemoryResponse {
            phys_addr: addr,
            obj_handle: req.obj_handle,
            section_handle: req.section_handle,
            size,
            virt_addr: req.virt_addr,
        }))
    }

    fn unmap_phys_mem(&self, mapping: PhysicalMemoryResponseBoxed) -> memflow_vdm::Result<()> {
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
                self.handle.handle(),
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

#[connector(name = "winio", description = "test")]
pub fn create_connector<'a>(_args: &ConnectorArgs) -> memflow::error::Result<VdmConnector<'a>> {
    let drv = WinIoDriver::open().map_err(|_| {
        Error(ErrorOrigin::Connector, ErrorKind::Uninitialized)
            .log_error("Unable to open a handle to the WinIo driver")
    })?;

    init_connector(Box::new(drv)).map_err(|_| {
        Error(ErrorOrigin::Connector, ErrorKind::Uninitialized)
            .log_error("Unable to initialize connector")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_phys_mem() -> memflow_vdm::Result<()> {
        const PAGE_SIZE: usize = 4096;

        let drv = WinIoDriver::open().map_err(memflow_vdm::Error::Windows)?;

        for addr in (0x0..0x10000u64).step_by(PAGE_SIZE) {
            let mapping = drv.map_phys_mem(addr, PAGE_SIZE)?;

            println!(
                "mapped physical memory from {:#X} -> {:#X} (size: {})",
                mapping.phys_addr(),
                mapping.virt_addr(),
                mapping.size(),
            );

            drv.unmap_phys_mem(mapping)?;
        }

        Ok(())
    }
}
