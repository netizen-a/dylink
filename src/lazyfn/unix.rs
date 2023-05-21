// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use std::ffi::{c_char, c_int, c_void, CStr};

use super::DefaultLinker;
use crate::LibHandle;

pub const RTLD_NOW: c_int = 0x2;
pub const RTLD_LOCAL: c_int = 0;
extern "C" {
	pub fn dlopen(filename: *const c_char, flag: c_int) -> *mut c_void;
	pub fn dlsym(handle: *mut c_void, symbol: *const c_char) -> Option<crate::FnPtr>;
}

impl crate::RTLinker for DefaultLinker {
	type Data = c_void;
	fn load_lib(lib_name: &CStr) -> LibHandle<'static, Self::Data> {
		unsafe {
			let result = dlopen(lib_name.as_ptr(), RTLD_NOW | RTLD_LOCAL);
			LibHandle::from(result.as_ref())
		}
	}
	fn load_sym(lib_handle: &LibHandle<'static, Self::Data>, fn_name: &CStr) -> Option<crate::FnPtr> {
		unsafe {
			dlsym(
				lib_handle
					.as_ref()
					.map(|r| r as *const _ as *mut c_void)
					.unwrap_or(std::ptr::null_mut()),
				fn_name.as_ptr(),
			)
		}
	}
}
