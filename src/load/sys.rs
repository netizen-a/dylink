// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use super::*;
use std::{
	ffi::{c_void, CString},
	io,
	sync::atomic::Ordering, os::windows::prelude::{IntoRawHandle, RawHandle},
};

unsafe impl Loader for System {
	/// If successful, increments reference count to shared library handle, and constructs `SystemLoader`.
	unsafe fn open(path: &str) -> io::Result<Self> {
		let handle: *mut c_void;
		#[cfg(unix)]
		{
			use crate::os::unix::*;
			let c_str = CString::new(path).unwrap();
			handle = dlopen(c_str.as_ptr(), RTLD_NOW | RTLD_LOCAL);
		}
		#[cfg(windows)]
		{
			use crate::os::win32::*;
			let wide_str: Vec<u16> = path.encode_utf16().chain(std::iter::once(0u16)).collect();
			handle = crate::os::win32::LoadLibraryExW(
				wide_str.as_ptr().cast(),
				std::ptr::null_mut(),
				LOAD_LIBRARY_SEARCH_DEFAULT_DIRS | LOAD_LIBRARY_SAFE_CURRENT_DIRS,
			);
		}
		if handle.is_null() {
			Err(io::Error::last_os_error())
		} else {
			Ok(Self(handle.into()))
		}
	}

	unsafe fn find(&self, symbol: &str) -> *const () {
		let c_str = CString::new(symbol).unwrap();
		crate::os::dlsym(self.0.load(Ordering::Relaxed), c_str.as_ptr().cast())
	}
}

impl System {
	/// Decrements reference counter to shared library. When reference counter hits zero the library is unloaded.
	/// # Errors
	/// May error depending on system call.
	pub unsafe fn close(self) -> io::Result<()> {
		let result = crate::os::dlclose(self.0.into_inner());
		if (cfg!(windows) && result == 0) || (cfg!(unix) && result != 0) {
			Err(io::Error::last_os_error())
		} else {
			Ok(())
		}
	}
}

// rust's std doesn't have a unix equivalent trait for IntoRawHandle

#[cfg(windows)]
impl IntoRawHandle for System {
	fn into_raw_handle(self) -> RawHandle {
		self.0.into_inner()
	}
}