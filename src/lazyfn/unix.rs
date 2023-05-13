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
	fn load_lib(lib_name: &CStr) -> LibHandle {
		unsafe { LibHandle(dlopen(lib_name.as_ptr(), RTLD_NOW | RTLD_LOCAL)) }
	}
	fn load_sym(lib_handle: &LibHandle, fn_name: &CStr) -> Option<crate::FnPtr> {
		unsafe { dlsym(lib_handle.0.cast_mut(), fn_name.as_ptr()) }
	}
}
