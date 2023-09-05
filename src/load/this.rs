// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
use super::*;
use crate::os::*;
use std::io;
use std::{ffi, sync::atomic::Ordering};

unsafe impl Loader for This {
	/// Does not increment reference count to handle.
	/// # Unix Platform
	/// On unix, `path` is ignored, and a default library handle is returned.
	///
	/// # Windows Platform
	/// On windows, `path` is used to load the library handle.
	unsafe fn open(path: &str) -> io::Result<Self> {
		#[cfg(unix)]
		{
			let _ = path;
			Ok(Self(unix::RTLD_DEFAULT.into()))
		}
		#[cfg(windows)]
		{
			use std::mem::MaybeUninit;
			let wide_str: Vec<u16> = path.encode_utf16().chain(std::iter::once(0u16)).collect();
			let wide_ptr = if path.is_empty() {
				std::ptr::null()
			} else {
				wide_str.as_ptr()
			};
			let mut handle = MaybeUninit::zeroed();
			let result = win32::GetModuleHandleExW(
				win32::GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
				wide_ptr,
				handle.as_mut_ptr(),
			);
			if result != 0 {
				Ok(Self(handle.assume_init().into()))
			} else {
				Err(io::Error::last_os_error())
			}
		}
	}
	unsafe fn find(&self, symbol: &str) -> *const () {
		let c_str = ffi::CString::new(symbol).unwrap();
		dlsym(self.0.load(Ordering::Relaxed), c_str.as_ptr())
	}
}
