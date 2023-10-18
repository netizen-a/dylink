#[cfg(any(unix, docsrs))]
pub mod unix;
#[cfg(any(windows, docsrs))]
pub(crate) mod windows;

pub(crate) type Handle = *mut std::ffi::c_void;
