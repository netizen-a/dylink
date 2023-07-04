// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use super::*;
use crate::os::*;
use std::ffi;

// internal type is opaque and managed by OS, so it's `Send` safe
unsafe impl Send for SelfLoader {}

unsafe impl Loader for SelfLoader {
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
	/// On unix,  `path` is ignored, and a default library handle is returned.
	///
	/// ### Windows Platform
	/// On windows, `path` is used to load the library handle.
	unsafe fn load_library(path: &str) -> Self {
		#[cfg(unix)]
		{
			let _ = path;
			Self(unix::RTLD_DEFAULT)
		}
		#[cfg(windows)]
		{
			if path.is_empty() {
				Self(win32::GetModuleHandleW(core::ptr::null_mut()))
			} else {
				let wide_str: Vec<u16> = path
					.encode_utf16()
					.chain(core::iter::once(0u16))
					.collect();
				Self(win32::GetModuleHandleW(wide_str.as_ptr()))
			}
		}
	}
	unsafe fn find_symbol(&self, symbol: &str) -> FnAddr {
		let c_str = ffi::CString::new(symbol).unwrap();
		dlsym(self.0, c_str.as_ptr())
	}
}
