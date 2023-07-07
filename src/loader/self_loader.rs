// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use super::*;
use crate::os::*;
use std::{ffi, mem::MaybeUninit};

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
			let wide_str: Vec<u16> = path
					.encode_utf16()
					.chain(core::iter::once(0u16))
					.collect();
			let wide_ptr = if path.is_empty() {
				std::ptr::null()
			} else {
				wide_str.as_ptr()
			};
			let mut handle = MaybeUninit::zeroed();
			let _ = win32::GetModuleHandleExW(
				win32::GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
				wide_ptr,
				handle.as_mut_ptr()
			);
			Self(handle.assume_init())
		}
	}
	unsafe fn find_symbol(&self, symbol: &str) -> SymAddr {
		let c_str = ffi::CString::new(symbol).unwrap();
		dlsym(self.0, c_str.as_ptr())
	}
}
