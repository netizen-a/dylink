// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

// The windows API conventions are kept deliberately, so it's easier to refer to references.
#![allow(clippy::upper_case_acronyms)]

use super::DefaultLinker;
use crate::LibHandle;
use std::ffi;
use std::os::windows::raw::HANDLE;

pub type HMODULE = HANDLE;
pub type PCSTR = *const ffi::c_char;
pub type PCWSTR = *const u16;
pub const LOAD_LIBRARY_SEARCH_DEFAULT_DIRS: u32 = 4096u32;
extern "system" {
	pub fn LoadLibraryExW(lplibfilename: PCWSTR, hfile: HANDLE, dwflags: u32) -> HMODULE;
	pub fn GetProcAddress(hmodule: HMODULE, lpprocname: PCSTR) -> Option<crate::FnPtr>;
}

impl crate::RTLinker for DefaultLinker {
	type Data = ffi::c_void;
	fn load_lib(lib_name: &ffi::CStr) -> LibHandle<'static, Self::Data> {
		let wide_str: Vec<u16> = lib_name
			.to_string_lossy()
			.encode_utf16()
			.chain(std::iter::once(0u16))
			.collect();
		let result = unsafe {
			// miri hates this function, but it works fine.
			LoadLibraryExW(
				wide_str.as_ptr().cast(),
				std::ptr::null_mut(),
				LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
			)
		};
		LibHandle::from(unsafe { result.as_ref() })
	}
	fn load_sym(
		lib_handle: &LibHandle<'static, Self::Data>,
		fn_name: &ffi::CStr,
	) -> Option<crate::FnPtr> {
		unsafe {
			GetProcAddress(
				lib_handle
					.as_ref()
					.map(|r| r as *const _ as *mut ffi::c_void)
					.unwrap_or(std::ptr::null_mut()),
				fn_name.as_ptr().cast(),
			)
		}
	}
}

#[test]
fn test_win32_macro_linker() {
	extern crate self as dylink;
	#[dylink::dylink(name = "Kernel32.dll", strip = true, linker=DefaultLinker)]
	extern "stdcall" {
		fn SetLastError(_: u32);
	}

	// macro output: function
	#[dylink::dylink(name = "Kernel32.dll", strip = false, linker=DefaultLinker)]
	extern "C" {
		fn GetLastError() -> u32;
	}

	unsafe {
		// static variable has crappy documentation, but can be use for library induction.
		match SetLastError.try_link() {
			Ok(f) => f(53),
			Err(e) => panic!("{}", e),
		}
		assert_eq!(GetLastError(), 53);
	}
}
