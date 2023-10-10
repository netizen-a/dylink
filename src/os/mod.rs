#[cfg(any(unix, docsrs))]
pub mod unix;
#[cfg(any(windows, docsrs))]
pub mod windows;

pub(crate) type Handle = *mut std::ffi::c_void;

extern "C" {
	fn wcslen(buf: *const u16) -> usize;
}
