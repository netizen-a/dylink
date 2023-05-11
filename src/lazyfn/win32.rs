use super::DefaultLinker;

use std::os::windows::raw::HANDLE;

pub type HMODULE = HANDLE;
pub type PCSTR = *const std::ffi::c_char;
pub type PCWSTR = *const u16;
pub const LOAD_LIBRARY_SEARCH_DEFAULT_DIRS: u32 = 4096u32;
extern "system" {
	pub fn LoadLibraryExW(lplibfilename: PCWSTR, hfile: HANDLE, dwflags: u32) -> HMODULE;
	pub fn GetProcAddress(hmodule: HMODULE, lpprocname: PCSTR) -> Option<crate::FnPtr>;
}

impl crate::RTLinker for DefaultLinker {
	fn load_lib(lib_name: &std::ffi::CStr) -> super::LibHandle {
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
		super::LibHandle(result)
	}
	fn load_sym(lib_handle: &super::LibHandle, fn_name: &std::ffi::CStr) -> Option<crate::FnPtr> {
		unsafe { GetProcAddress(lib_handle.0, fn_name.as_ptr().cast()) }
	}	
}