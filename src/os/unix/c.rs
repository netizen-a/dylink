// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
#![allow(non_camel_case_types)]

use std::ffi;

#[repr(C)]
pub struct Dl_info {
	pub dli_fname: *const ffi::c_char,
	pub dli_fbase: *mut ffi::c_void,
	pub dli_sname: *const ffi::c_char,
	pub dli_saddr: *mut ffi::c_void,
}

extern "C-unwind" {
	pub fn dlopen(filename: *const ffi::c_char, flag: ffi::c_int) -> *mut ffi::c_void;
	pub fn dlerror() -> *const ffi::c_char;
	pub fn dlsym(handle: *mut ffi::c_void, symbol: *const ffi::c_char) -> *const ffi::c_void;
	pub fn dlclose(hlibmodule: *mut ffi::c_void) -> ffi::c_int;

	#[cfg(not(target_os = "aix"))]
	pub fn dladdr(addr: *const ffi::c_void, info: *mut Dl_info) -> ffi::c_int;
	#[cfg(target_env = "gnu")]
	pub fn dlinfo(
		handle: *mut ffi::c_void,
		request: ffi::c_int,
		info: *mut ffi::c_void,
	) -> ffi::c_int;
}

// opaque type because I don't care about the contents atm.
#[cfg(target_os = "macos")]
#[repr(C)]
pub struct mach_header {
    _data: [u8; 0],
    _marker:
        core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

#[cfg(target_os = "macos")]
pub type PfnImageCallback = extern "C" fn(mh: *const mach_header, vmaddr_slide: isize);

#[cfg(target_os = "macos")]
extern "C-unwind" {
	pub fn _dyld_get_image_name(image_index: u32) -> *const ffi::c_char;
	pub fn _dyld_register_func_for_add_image(func: PfnImageCallback);
	pub fn _dyld_register_func_for_remove_image(func: PfnImageCallback);
}

pub const RTLD_LOCAL: ffi::c_int = 0;
pub const RTLD_NOW: ffi::c_int = 0x2;
#[cfg(target_os = "macos")]
pub const RTLD_NOLOAD: ffi::c_int = 0x4;

#[cfg(target_env = "gnu")]
pub const RTLD_DI_LINKMAP: ffi::c_int = 2;

#[cfg(target_env = "gnu")]
pub type ElfW_Addr = usize;

#[cfg(target_env = "gnu")]
#[repr(C)]
pub struct ElfW_Dyn {
	d_tag: usize,
	d_un: usize,
}

#[cfg(target_env = "gnu")]
#[repr(C)]
pub struct link_map {
	pub l_addr: ElfW_Addr,
	pub l_name: *mut ffi::c_char,
	pub l_ld: *mut ElfW_Dyn,
	pub l_next: *mut link_map,
	pub l_prev: *mut link_map,
	_marker: std::marker::PhantomPinned,
}
