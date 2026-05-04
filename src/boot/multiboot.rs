use core::ffi::c_void;

pub type EfiHandle = *mut c_void;
pub type EfiStatus = usize;

#[repr(C)]
pub struct SystemTable {
    _private: [u8; 0],
}
