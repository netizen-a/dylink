#![allow(clippy::upper_case_acronyms)]

use std::ffi;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::raw;
use std::path;
use std::ptr;

use crate::Library;
use crate::Sym;

type HMODULE = raw::HANDLE;
type PCWSTR = *const u16;
type PWSTR = *mut u16;
type BOOL = i32;
type DWORD = u32;

//pub const GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT: u32 = 0x00000002u32;
extern "stdcall" {
	fn LoadLibraryExW(lplibfilename: PCWSTR, hfile: raw::HANDLE, dwflags: u32) -> HMODULE;
	fn GetModuleHandleExW(dwflags: u32, lpmodulename: PCWSTR, phmodule: *mut HMODULE) -> BOOL;
	fn GetProcAddress(handle: *mut ffi::c_void, symbol: *const ffi::c_char) -> *const ffi::c_void;
	fn FreeLibrary(hlibmodule: *mut ffi::c_void) -> ffi::c_int;
	fn GetModuleFileNameW(hmodule: HMODULE, lpfilename: PWSTR, nsize: DWORD) -> DWORD;
	fn FreeLibraryAndExitThread(hLibModule: HMODULE, dwExitCode: DWORD) -> !;
}

pub trait LibraryExt {
	fn get_path(&mut self) -> io::Result<path::PathBuf>;
}

impl LibraryExt for Library {
	fn get_path(&mut self) -> io::Result<path::PathBuf> {
		use std::os::windows::ffi::OsStringExt;
		const MAX_PATH: usize = 260;
		let mut file_name = vec![0u16; MAX_PATH];
		loop {
			let _ = unsafe {
				GetModuleFileNameW(
					*self.0.get_mut(),
					file_name.as_mut_ptr(),
					file_name.len() as DWORD,
				)
			};
			let last_error = io::Error::last_os_error();
			match unsafe { last_error.raw_os_error().unwrap_unchecked() } {
				0 => {
					let os_str = ffi::OsString::from_wide(&file_name);
					break Ok(os_str.into());
				}
				0x7A => file_name.resize(file_name.len() * 2, 0),
				_ => break Err(last_error),
			}
		}
	}
}

pub(crate) unsafe fn dylib_open<P: AsRef<ffi::OsStr>>(path: P) -> io::Result<*mut ffi::c_void> {
	let os_str = path.as_ref();
	let wide_str: Vec<u16> = os_str.encode_wide().chain(std::iter::once(0u16)).collect();
	let handle: *mut ffi::c_void =
		LoadLibraryExW(wide_str.as_ptr().cast(), std::ptr::null_mut(), 0);
	if handle.is_null() {
		Err(io::Error::last_os_error())
	} else {
		Ok(handle)
	}
}

pub(crate) unsafe fn dylib_this() -> io::Result<*mut ffi::c_void> {
	let mut handle: *mut ffi::c_void = ptr::null_mut();
	let result = GetModuleHandleExW(0, ptr::null(), &mut handle);
	if result == 0 {
		Err(io::Error::last_os_error())
	} else {
		Ok(handle)
	}
}

pub(crate) unsafe fn dylib_close(lib_handle: *mut ffi::c_void) -> io::Result<()> {
	let result = FreeLibrary(lib_handle);
	if result == 0 {
		Err(io::Error::last_os_error())
	} else {
		Ok(())
	}
}

pub(crate) unsafe fn dylib_symbol(lib_handle: *mut ffi::c_void, name: &str) -> io::Result<&Sym> {
	let c_str = ffi::CString::new(name).unwrap();
	let addr: *const () = unsafe { GetProcAddress(lib_handle, c_str.as_ptr().cast()).cast() };
	if addr.is_null() {
		Err(io::Error::last_os_error())
	} else {
		Ok(addr.cast::<Sym>().as_ref().unwrap_unchecked())
	}
}

pub(crate) unsafe fn dylib_close_and_exit(lib_handle: *mut ffi::c_void, exit_code: u32) -> ! {
	FreeLibraryAndExitThread(lib_handle, exit_code)
}
