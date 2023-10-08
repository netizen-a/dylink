use std::ffi;

#[repr(C)]
pub struct Dl_info {
	pub dli_fname: *const ffi::c_char,
	pub dli_fbase: *mut ffi::c_void,
	pub dli_sname: *const ffi::c_char,
	pub dli_saddr: *mut ffi::c_void,
}

extern "C" {
	pub fn dlopen(filename: *const ffi::c_char, flag: ffi::c_int) -> *mut ffi::c_void;
	pub fn dlerror() -> *const ffi::c_char;
	pub fn dlsym(handle: *mut ffi::c_void, symbol: *const ffi::c_char) -> *const ffi::c_void;
	pub fn dlclose(hlibmodule: *mut ffi::c_void) -> ffi::c_int;

	#[cfg(not(target_os = "aix"))]
	pub fn dladdr(addr: *const ffi::c_void, info: *mut Dl_info) -> ffi::c_int;
}

pub const RTLD_NOW: ffi::c_int = 0x2;
pub const RTLD_NOLOAD: ffi::c_int = 0x4;
