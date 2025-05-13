use std::ffi::c_void;

use memflow::types::{Address, PhysicalAddress};

use memflow_vdm::*;

use windows::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, HANDLE};

use windows::Win32::Storage::FileSystem::{
    CreateFileA, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_MODE, OPEN_EXISTING,
};

use windows::Win32::System::IO::DeviceIoControl;
use windows::core::{PCSTR, s};

const DEVICE_PATH: PCSTR = s!(r"\\.\WinIo");

#[repr(u32)]
enum IoControlCode {
    MapPhysicalMemory = 0x80102040,
    UnmapPhysicalMemory = 0x80102044,
}


#[repr(C)]
pub struct PhysicalMemoryIoRequest {
    size: u64,
    phys_addr: u64,
    sec_handle: *mut c_void,
    virt_addr: u64,
    obj_handle: *mut c_void,
}

impl Default for PhysicalMemoryIoRequest {
    fn default() -> Self {
        PhysicalMemoryIoRequest {
            size: 0,
            phys_addr: 0,
            sec_handle: std::ptr::null_mut(),
            virt_addr: 0,
            obj_handle: std::ptr::null_mut(),
        }
    }
}

impl PhysicalMemoryMapping for PhysicalMemoryIoRequest {
    #[inline]
    fn phys_addr(&self) -> PhysicalAddress {
        self.phys_addr.into()
    }

    #[inline]
    fn size(&self) -> usize {
        self.size as _
    }

    #[inline]
    fn virt_addr(&self) -> Address {
        self.virt_addr.into()
    }
}

unsafe impl Send for PhysicalMemoryIoRequest {}
unsafe impl Sync for PhysicalMemoryIoRequest {}

#[derive(VdmDriver)]
#[cfg_attr(
    feature = "auto-start",
    connector(
        conn_name = "winio",
        use_env_vars = true,
        service_name = "winio",
        driver_path = r"C:\winio64.sys"
    )
)]
#[cfg_attr(not(feature = "auto-start"), connector(conn_name = "winio"))]
pub struct WinIoDriver {
    handle: HANDLE,
}

impl WinIoDriver {
    pub fn open() -> Result<Self> {
        let handle = unsafe {
            CreateFileA(
                DEVICE_PATH,
                GENERIC_READ.0 | GENERIC_WRITE.0,
                FILE_SHARE_MODE(0),
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
            .map_err(|e| Error::OpenDevice {
                device_path: DEVICE_PATH.to_string().unwrap(),
                source: e,
            })?
        };

        Ok(Self { handle })
    }
}

impl PhysicalMemory for WinIoDriver {
    type Response = PhysicalMemoryIoRequest;

    fn map_physical_memory(
        &self,
        phys_addr: PhysicalAddress,
        size: usize,
    ) -> Result<Self::Response> {
        let mut req = PhysicalMemoryIoRequest {
            size: size as _,
            phys_addr: phys_addr.to_umem(),
            ..Default::default()
        };

        unsafe {
            DeviceIoControl(
                self.handle,
                IoControlCode::MapPhysicalMemory as _,
                Some(&req as *const _ as *const _),
                size_of::<PhysicalMemoryIoRequest>() as _,
                Some(&mut req as *mut _ as *mut _),
                size_of::<PhysicalMemoryIoRequest>() as _,
                None,
                None,
            )
            .map_err(|_| Error::MapPhysicalMemory { addr: phys_addr })?
        }

        Ok(req)
    }

    fn unmap_physical_memory(&self, mapping: &Self::Response) -> Result<()> {
        let req = PhysicalMemoryIoRequest {
            sec_handle: mapping.sec_handle,
            virt_addr: mapping.virt_addr().to_umem(),
            obj_handle: mapping.obj_handle,
            ..Default::default()
        };

        unsafe {
            DeviceIoControl(
                self.handle,
                IoControlCode::UnmapPhysicalMemory as _,
                Some(&req as *const _ as *const _),
                size_of::<PhysicalMemoryIoRequest>() as _,
                None,
                0,
                None,
                None,
            )
            .map_err(|_| Error::UnmapPhysicalMemory {
                addr: mapping.virt_addr(),
            })
        }
    }
}

unsafe impl Send for WinIoDriver {}
unsafe impl Sync for WinIoDriver {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_phys_mem() -> Result<()> {
        const PAGE_SIZE: usize = 4096;

        let drv = WinIoDriver::open()?;

        (0u64..0x10000)
            .step_by(PAGE_SIZE)
            .map(PhysicalAddress::from)
            .try_for_each(|phys_addr| -> Result<()> {
                let mapping = drv.map_physical_memory(phys_addr, PAGE_SIZE)?;

                println!(
                    "mapped physical memory from {:#X} -> {:#X} ({} bytes)",
                    mapping.phys_addr(),
                    mapping.virt_addr(),
                    mapping.size(),
                );

                drv.unmap_physical_memory(&mapping)
            })
    }
}
