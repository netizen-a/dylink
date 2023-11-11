// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
#![allow(non_camel_case_types)]

use std::{ffi, ptr};

#[cfg(target_os = "macos")]
type cpu_type_t = ffi::c_int;
#[cfg(target_os = "macos")]
type cpu_subtype_t = ffi::c_int;

#[cfg(target_os = "macos")]
#[repr(C)]
pub struct mach_header {
	pub magic: ffi::c_ulong,       /* mach magic number identifier */
	pub cputype: cpu_type_t,       /* cpu specifier */
	pub cpusubtype: cpu_subtype_t, /* machine specifier */
	pub filetype: ffi::c_ulong,    /* type of file */
	pub ncmds: ffi::c_ulong,       /* number of load commands */
	pub sizeofcmds: ffi::c_ulong,  /* the size of all the load commands */
	pub flags: ffi::c_ulong,       /* flags */
}

#[cfg(target_os = "macos")]
pub type PfnImageCallback = extern "C" fn(mh: *const mach_header, vmaddr_slide: isize);

#[cfg(target_os = "macos")]
extern "C" {
	pub fn _dyld_get_image_name(image_index: u32) -> *const ffi::c_char;
	pub fn _dyld_register_func_for_add_image(func: PfnImageCallback);
	pub fn _dyld_register_func_for_remove_image(func: PfnImageCallback);
	// returns base address
	pub fn _dyld_get_image_header(image_index: u32) -> *const mach_header;
}

#[cfg(target_env = "gnu")]
#[derive(Debug)]
#[repr(C)]
pub struct ElfW_Dyn {
	d_tag: usize,
	d_un: usize,
}

#[cfg(target_env = "gnu")]
#[derive(Debug)]
#[repr(C)]
pub struct link_map {
	pub l_addr: usize,
	pub l_name: *mut ffi::c_char,
	pub l_ld: *mut ElfW_Dyn,
	pub l_next: *mut link_map,
	pub l_prev: *mut link_map,
	_marker: std::marker::PhantomPinned,
}
