use std::marker::PhantomData;
use std::os::windows::ffi::{OsStrExt, OsStringExt};

use std::os::windows::prelude::*;
use std::{ffi, io, path, ptr};

use super::Handle;
use crate::{Library, Symbol};

mod c;

fn to_wide(path: &ffi::OsStr) -> Vec<u16> {
	path.encode_wide().chain(std::iter::once(0u16)).collect()
}

#[inline]
pub(crate) unsafe fn dylib_open(path: &ffi::OsStr) -> io::Result<Handle> {
	let wide_str: Vec<u16> = to_wide(path);
	let handle = c::LoadLibraryExW(wide_str.as_ptr(), ptr::null_mut(), 0);
	if handle.is_null() {
		Err(io::Error::last_os_error())
	} else {
		Ok(handle.cast())
	}
}

#[inline]
pub(crate) unsafe fn dylib_this() -> io::Result<Handle> {
	let mut handle: *mut ffi::c_void = ptr::null_mut();
	let result = c::GetModuleHandleExW(0, ptr::null(), &mut handle);
	if result == 0 {
		Err(io::Error::last_os_error())
	} else {
		Ok(handle.cast())
	}
}

#[inline]
pub(crate) unsafe fn dylib_close(lib_handle: Handle) -> io::Result<()> {
	if c::FreeLibrary(lib_handle.cast()) == 0 {
		Err(io::Error::last_os_error())
	} else {
		Ok(())
	}
}

#[inline]
pub(crate) unsafe fn dylib_symbol<'a>(lib_handle: Handle, name: &str) -> io::Result<Symbol<'a>> {
	let c_str = ffi::CString::new(name).unwrap();
	let addr: *const ffi::c_void = unsafe { c::GetProcAddress(lib_handle.cast(), c_str.as_ptr()) };
	if addr.is_null() {
		Err(io::Error::last_os_error())
	} else {
		Ok(Symbol(addr.cast_mut(), PhantomData))
	}
}

#[inline]
pub(crate) unsafe fn dylib_close_and_exit(lib_handle: Handle, exit_code: i32) -> ! {
	c::FreeLibraryAndExitThread(lib_handle.cast(), exit_code as u32)
}

#[cfg(feature = "unstable")]
pub(crate) unsafe fn dylib_is_loaded(path: &ffi::OsStr) -> bool {
	let wide_str: Vec<u16> = to_wide(path);
	let mut handle = ptr::null_mut();
	let _ = c::GetModuleHandleExW(
		c::GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
		wide_str.as_ptr(),
		&mut handle,
	);
	!handle.is_null()
}

impl AsHandle for Library {
	fn as_handle(&self) -> BorrowedHandle<'_> {
		unsafe { BorrowedHandle::borrow_raw(self as *const _ as *mut _) }
	}
}

impl AsRawHandle for Library {
	fn as_raw_handle(&self) -> RawHandle {
		self as *const _ as *mut _
	}
}

pub(crate) unsafe fn dylib_path(handle: Handle) -> io::Result<path::PathBuf> {
	const MAX_PATH: usize = 260;
	let mut file_name = vec![0u16; MAX_PATH];
	loop {
		let _ = c::GetModuleFileNameW(handle, file_name.as_mut_ptr(), file_name.len() as c::DWORD);
		let last_error = io::Error::last_os_error();
		match last_error.raw_os_error().unwrap_unchecked() {
			0 => {
				// must be truncated for metadata to work.
				if let Some(new_len) = file_name.iter().rposition(|a| *a != 0) {
					file_name.truncate(new_len + 1)
				}
				let os_str = ffi::OsString::from_wide(&file_name);
				break Ok(os_str.into());
			}
			0x7A => file_name.resize(file_name.len() * 2, 0),
			_ => break Err(last_error),
		}
	}
}

pub(crate) unsafe fn base_addr(symbol: &Symbol) -> io::Result<*const ffi::c_void> {
	let mut handle = ptr::null_mut();
	let result = c::GetModuleHandleExW(
		c::GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT
		| c::GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
		symbol.cast(),
		&mut handle,
	);
	if result == 0 {
		Err(io::Error::last_os_error())
	} else {
		Ok(handle)
	}
}
