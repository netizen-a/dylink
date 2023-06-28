// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use super::*;
use crate::os::*;
use core::ffi::CStr;

// internal type is opaque and managed by OS, so it's `Send` safe
unsafe impl Send for SelfLoader {}

impl Loader for SelfLoader {
	fn is_invalid(&self) -> bool {
		if cfg!(windows) {
			self.0.is_null()
		} else if cfg!(unix) {
			false
		} else {
			unreachable!("platform unsupported")
		}
	}
	/// Does not increment reference count to handle.
	/// ### Unix Platform
	/// On unix,  `_lib_name` is ignored, and a default library handle is returned.
	///
	/// ### Windows Platform
	/// On windows, `_lib_name` is used to load the library handle.
	fn load_lib(_lib_name: &'static CStr) -> Self {
		#[cfg(unix)]
		{
			Self(unix::RTLD_DEFAULT)
		}
		#[cfg(windows)]
		{
			// FIXME: when `CStr::is_empty` is stable, replace `to_bytes().is_empty()`.
			if _lib_name.to_bytes().is_empty() {
				unsafe { Self(win32::GetModuleHandleW(core::ptr::null_mut())) }
			} else {
				let wide_str: Vec<u16> = _lib_name
					.to_string_lossy()
					.encode_utf16()
					.chain(core::iter::once(0u16))
					.collect();
				unsafe { Self(win32::GetModuleHandleW(wide_str.as_ptr())) }
			}
		}
	}
	fn load_sym(&self, fn_name: &CStr) -> FnAddr {
		unsafe { dlsym(self.0, fn_name.as_ptr()) }
	}
}
