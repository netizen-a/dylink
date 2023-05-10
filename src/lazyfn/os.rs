#[cfg(windows)]
pub mod win32 {
	use core::ffi::{c_char, c_void};
	type HANDLE = *mut c_void;

	pub type HMODULE = HANDLE;
	pub type PCSTR = *const c_char;
	pub type PCWSTR = *const u16;
	pub const LOAD_LIBRARY_SEARCH_DEFAULT_DIRS: u32 = 4096u32;

	extern "system" {
		pub fn LoadLibraryExW(lplibfilename: PCWSTR, hfile: HANDLE, dwflags: u32) -> HMODULE;
		pub fn GetProcAddress(hmodule: HMODULE, lpprocname: PCSTR) -> Option<crate::FnPtr>;
	}
}

#[cfg(unix)]
pub mod unix {
	use core::ffi::{c_char, c_int, c_void};
	pub const RTLD_NOW: c_int = 0x2;
	pub const RTLD_LOCAL: c_int = 0;

	extern "C" {
		pub fn dlopen(filename: *const c_char, flag: c_int) -> *mut c_void;
		pub fn dlsym(handle: *mut c_void, symbol: *const c_char) -> Option<crate::FnPtr>;
	}
}
