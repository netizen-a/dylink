// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use std::ffi::{c_char, c_void};
// yes this is cursed, and no I'm not changing it.
extern "system" {
	#[cfg_attr(windows, link_name = "GetProcAddress")]
	#[cfg_attr(unix, link_name = "dlsym")]
	pub fn dlsym(handle: *mut c_void, symbol: *const c_char) -> crate::FnAddr;
	#[cfg(feature = "unload")]
	#[cfg_attr(windows, link_name = "FreeLibrary")]
	#[cfg_attr(unix, link_name = "dlclose")]
	pub fn dlclose(hlibmodule: *mut c_void) -> std::ffi::c_int;
}

#[cfg(windows)]
pub mod win32 {
	use std::os::windows::raw::HANDLE;
	type HMODULE = HANDLE;
	type PCWSTR = *const u16;
	pub const LOAD_LIBRARY_SEARCH_DEFAULT_DIRS: u32 = 0x00001000u32;
	pub const LOAD_LIBRARY_SAFE_CURRENT_DIRS: u32 = 0x00002000u32;
	extern "stdcall" {
		pub fn LoadLibraryExW(lplibfilename: PCWSTR, hfile: HANDLE, dwflags: u32) -> HMODULE;
		pub fn GetModuleHandleW(lpmodulename: PCWSTR) -> HMODULE;
	}
}

#[cfg(unix)]
pub mod unix {
	use std::ffi::{c_char, c_int, c_void};
	pub const RTLD_NOW: c_int = 0x2;
	pub const RTLD_LOCAL: c_int = 0;
	pub const RTLD_DEFAULT: *mut c_void = std::ptr::null_mut();
	extern "C" {
		pub fn dlopen(filename: *const c_char, flag: c_int) -> *mut c_void;
	}
}
