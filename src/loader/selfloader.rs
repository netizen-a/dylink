// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
use super::*;
use crate::os::*;
use std::ffi::CStr;

#[doc(hidden)]
pub struct SelfHandle(*mut std::ffi::c_void);
unsafe impl Send for SelfHandle {}

#[cfg(windows)]
impl crate::loader::LibHandle for SelfHandle {
	fn is_invalid(&self) -> bool {
		self.0.is_null()
	}
}
#[cfg(unix)]
impl crate::loader::LibHandle for SelfHandle {
	fn is_invalid(&self) -> bool {
		// lib handle is ignored, so is_invalid must always return false
		false
	}
}

impl Loader<'_> for SelfLoader {
	type Handle = SelfHandle;
	#[cfg(unix)]
	fn load_lib(_: &CStr) -> Self::Handle {
		SelfHandle(unix::RTLD_DEFAULT)
	}
	#[cfg(windows)]
	fn load_lib(lib_name: &'static CStr) -> Self::Handle {
		// FIXME: when `CStr::is_empty` is stable, replace `to_bytes().is_empty()`.
		if lib_name.to_bytes().is_empty() {
			unsafe { SelfHandle(win32::GetModuleHandleW(std::ptr::null_mut())) }
		} else {
			let wide_str: Vec<u16> = lib_name
				.to_string_lossy()
				.encode_utf16()
				.chain(std::iter::once(0u16))
				.collect();
			unsafe { SelfHandle(win32::GetModuleHandleW(wide_str.as_ptr())) }
		}
	}
	fn load_sym(lib_handle: &Self::Handle, fn_name: &CStr) -> FnAddr {
		unsafe { dlsym(lib_handle.0, fn_name.as_ptr()) }
	}
}
