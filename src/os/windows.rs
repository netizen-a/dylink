#![allow(clippy::upper_case_acronyms)]


use std::os::windows::raw;
use std::ffi;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::ptr;

use crate::Sym;

type HMODULE = raw::HANDLE;
type PCWSTR = *const u16;
type BOOL = i32;
//pub const GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT: u32 = 0x00000002u32;
extern "stdcall" {
	pub fn LoadLibraryExW(lplibfilename: PCWSTR, hfile: raw::HANDLE, dwflags: u32) -> HMODULE;
	pub fn GetModuleHandleExW(
		dwFlags: u32,
		lpmodulename: PCWSTR,
		phModule: *mut HMODULE,
	) -> BOOL;
    pub fn GetProcAddress(handle: *mut ffi::c_void, symbol: *const ffi::c_char) -> *const ffi::c_void;
    pub fn FreeLibrary(hlibmodule: *mut ffi::c_void) -> ffi::c_int;
	//pub fn SymInitializeW(
	//	hProcess: raw::HANDLE,
	//	UserSearchPath: PCWSTR,
	//	fInvadeProcess: BOOL,
	//) -> BOOL;
}

/*pub struct Symbols<'a>(Vec<Sym<'a>>);

impl <'a> Iterator for Symbols<'a> {
	type Item = Sym<'a>;
	fn next(&mut self) -> Option<Self::Item> {

	}
}

impl Drop for Symbols<'_> {
	fn drop(&mut self) {

	}
}

pub trait LibraryExt {
	fn symbols<'a>(&'a self) -> Symbols<'a> {

	}
}*/

pub unsafe fn dylib_open<P: AsRef<ffi::OsStr>>(path: P) -> io::Result<*mut ffi::c_void> {
    let handle: *mut ffi::c_void;
	let os_str = path.as_ref();
	let wide_str: Vec<u16> = os_str.encode_wide().chain(std::iter::once(0u16)).collect();
	handle = LoadLibraryExW(
		wide_str.as_ptr().cast(),
		std::ptr::null_mut(),
		0,
	);
	if handle.is_null() {
		Err(io::Error::last_os_error())
	} else {
		Ok(handle)
	}
}

pub unsafe fn dylib_this() -> io::Result<*mut ffi::c_void> {
	let mut handle: *mut ffi::c_void = ptr::null_mut();
	let result = GetModuleHandleExW(0, ptr::null(), &mut handle);
	if result == 0 {
		Err(io::Error::last_os_error())
	} else {
		Ok(handle)
	}
}

pub unsafe fn dylib_close(lib_handle: *mut ffi::c_void) -> io::Result<()> {
	let result = crate::os::windows::FreeLibrary(lib_handle);
    if result == 0 {
		Err(io::Error::last_os_error())
	} else {
		Ok(())
	}
}

pub unsafe fn dylib_symbol(lib_handle: *mut ffi::c_void, name: &str) -> io::Result<&Sym> {
    let c_str = ffi::CString::new(name).unwrap();
	let addr: *const () = unsafe {
		GetProcAddress(lib_handle, c_str.as_ptr().cast()).cast()
	};
	if addr.is_null() {
		// todo: use dlerror for unix
		Err(io::Error::last_os_error())
	} else {
		Ok(addr.cast::<Sym>().as_ref().unwrap_unchecked())
	}
}