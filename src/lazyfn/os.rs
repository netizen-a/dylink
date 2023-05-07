#[cfg(windows)]
pub mod win32 {
	use std::ffi;
	use std::os::windows::raw::HANDLE;

	pub type HMODULE = HANDLE;
	pub type PCSTR = *const ffi::c_char;
	pub type FARPROC = Option<crate::FnPtr>;
	pub type PCWSTR = *const u16;
	pub const LOAD_LIBRARY_SEARCH_DEFAULT_DIRS: u32 = 4096u32;

	extern "system" {
		pub fn GetProcAddress(hmodule: HMODULE, lpprocname: PCSTR) -> FARPROC;
		pub fn LoadLibraryExW(lplibfilename: PCWSTR, hfile: HANDLE, dwflags: u32) -> HMODULE;
	}
}

#[cfg(unix)]
pub mod unix {
	use std::ffi;
	pub const RTLD_NOW: ffi::c_int = 0x2;
	pub const RTLD_LOCAL: ffi::c_int = 0;

	extern "C" {
		pub fn dlopen(filename: *const ffi::c_char, flag: ffi::c_int) -> *mut ffi::c_void;
		pub fn dlsym(handle: *mut ffi::c_void, symbol: *const ffi::c_char) -> *mut ffi::c_void;
	}
}
