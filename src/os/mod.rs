#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod windows;

pub(crate) type Handle = *mut std::ffi::c_void;
