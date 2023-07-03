// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use core::ffi::c_int;

use super::*;

// internal type is opaque and managed by OS, so it's `Send` safe
unsafe impl Send for SysLoader {}

impl Loader for SysLoader {
	fn is_invalid(&self) -> bool {
		self.0.is_null()
	}
	//type Handle = SysHandle;
	/// Increments reference count to handle, and returns handle if successful.
	unsafe fn load_library(lib_name: &'static ffi::CStr) -> Self {
		#[cfg(unix)]
		{
			use crate::os::unix::*;
			Self(dlopen(lib_name.as_ptr(), RTLD_NOW | RTLD_LOCAL))
		}
		#[cfg(windows)]
		{
			use crate::os::win32::*;
			let wide_str: Vec<u16> = lib_name
				.to_string_lossy()
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

	unsafe fn find_symbol(&self, fn_name: &'static ffi::CStr) -> crate::FnAddr {
		crate::os::dlsym(self.0, fn_name.as_ptr().cast())
	}
}

#[cfg(any(feature = "unload", doc))]
impl Unloadable for SysLoader {
	type Error = c_int;
	unsafe fn unload(&self) -> Result<(), Self::Error> {
		let result = crate::os::dlclose(self.0);
		if (cfg!(windows) && result == 0) || (cfg!(unix) && result != 0) {
			Err(result)
		} else {
			Ok(())
		}
	}
}
