// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use std::{io, ffi::CString};
use super::*;

// internal type is opaque and managed by OS, so it's `Send` safe
unsafe impl Send for SystemLoader {}

unsafe impl Loader for SystemLoader {
	fn is_invalid(&self) -> bool {
		self.0.is_null()
	}
	/// Increments reference count to handle, and returns handle if successful.
	unsafe fn load_library(path: &str) -> Self {
		#[cfg(unix)]
		{
			use crate::os::unix::*;
			let c_str = CString::new(path).unwrap();
			Self(dlopen(c_str.as_ptr(), RTLD_NOW | RTLD_LOCAL))
		}
		#[cfg(windows)]
		{
			use crate::os::win32::*;
			let wide_str: Vec<u16> = path
				.encode_utf16()
				.chain(core::iter::once(0u16))
				.collect();

			Self(crate::os::win32::LoadLibraryExW(
				wide_str.as_ptr().cast(),
				core::ptr::null_mut(),
				LOAD_LIBRARY_SEARCH_DEFAULT_DIRS | LOAD_LIBRARY_SAFE_CURRENT_DIRS,
			))
		}
	}

	unsafe fn find_symbol(&self, symbol: &str) -> crate::FnAddr {
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
			#[cfg(windows)] {
				// windows dumps *all* error info into this call.
				Err(io::Error::last_os_error())
			}
			#[cfg(unix)] {
				// unix uses dlerror to for error handling, but it's
				// not MT-safety guarenteed, so I can't use it.
				Err(io::Error::new(
					io::ErrorKind::Other,
					"Unknown Error. Call `dlerror` for more information"
				))
			}
		} else {
			Ok(())
		}
	}
}
