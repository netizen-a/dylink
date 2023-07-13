// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use super::*;
use std::{
	ffi::{c_void, CString},
	io,
};

// internal type is opaque and managed by OS, so it's `Send` safe
unsafe impl Send for SystemLoader {}

unsafe impl Loader for SystemLoader {
	/// If successful, increments reference count to shared library handle, and constructs `SystemLoader`.
	unsafe fn open(path: &str) -> Option<Self> {
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
			None
		} else {
			Some(Self(handle))
		}
	}

	unsafe fn find_symbol(&self, symbol: &str) -> SymAddr {
		let c_str = CString::new(symbol).unwrap();
		crate::os::dlsym(self.0, c_str.as_ptr().cast())
	}
}

unsafe impl Close for SystemLoader {
	/// Decrements reference counter to shared library. When reference counter hits zero the library is unloaded.
	/// ## Errors
	/// May error depending on system call.
	unsafe fn close(self) -> io::Result<()> {
		let result = crate::os::dlclose(self.0);
		if (cfg!(windows) && result == 0) || (cfg!(unix) && result != 0) {
			#[cfg(windows)]
			{
				// windows dumps *all* error info into this call.
				Err(io::Error::last_os_error())
			}
			#[cfg(unix)]
			{
				// unix uses dlerror to for error handling, but it's
				// not MT-safety guarenteed, so I can't use it.
				Err(io::Error::new(
					io::ErrorKind::Other,
					"Unknown Error. Call `dlerror` for more information",
				))
			}
		} else {
			Ok(())
		}
	}
}
