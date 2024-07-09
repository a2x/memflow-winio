use std::ffi::c_void;
use std::ptr;

use windows::Win32::Foundation::HANDLE;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct RawHandle(pub(crate) *mut c_void);

impl RawHandle {
    #[inline]
    pub const fn new(ptr: *mut c_void) -> Self {
        Self(ptr)
    }

    #[inline]
    pub const fn null() -> Self {
        Self(ptr::null_mut())
    }

    #[inline]
    pub fn handle(&self) -> HANDLE {
        (*self).into()
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        !self.is_null()
    }
}

impl Default for RawHandle {
    #[inline]
    fn default() -> Self {
        Self::null()
    }
}

impl From<HANDLE> for RawHandle {
    #[inline]
    fn from(handle: HANDLE) -> Self {
        Self(handle.0 as _)
    }
}

impl From<RawHandle> for HANDLE {
    #[inline]
    fn from(handle: RawHandle) -> Self {
        Self(handle.0 as _)
    }
}

unsafe impl Send for RawHandle {}
unsafe impl Sync for RawHandle {}
