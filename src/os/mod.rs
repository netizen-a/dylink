#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod windows;

// an owned handle may not be null
pub(crate) type Handle = std::ptr::NonNull<std::ffi::c_void>;
