use std::os::windows::ffi::{OsStrExt, OsStringExt};

use std::os::windows::prelude::*;
use std::{ffi, io, path, ptr};

use super::Handle;
use crate::sealed::Sealed;
use crate::Library;
use crate::Sym;

mod c;
mod dbghelp;

pub use dbghelp::SymbolInfo;


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
		Ok(handle)
	}
}

#[inline]
pub(crate) unsafe fn dylib_this() -> io::Result<Handle> {
	let mut handle: *mut ffi::c_void = ptr::null_mut();
	let result = c::GetModuleHandleExW(0, ptr::null(), &mut handle);
	if result == 0 {
		Err(io::Error::last_os_error())
	} else {
		Ok(handle)
	}
}

#[inline]
pub(crate) unsafe fn dylib_close(lib_handle: Handle) -> io::Result<()> {
	if c::FreeLibrary(lib_handle) == 0 {
		Err(io::Error::last_os_error())
	} else {
		Ok(())
	}
}

#[inline]
pub(crate) unsafe fn dylib_symbol<'a>(lib_handle: Handle, name: &str) -> io::Result<&'a Sym> {
	let c_str = ffi::CString::new(name).unwrap();
	let addr: *const () = unsafe { c::GetProcAddress(lib_handle, c_str.as_ptr()).cast() };
	if addr.is_null() {
		Err(io::Error::last_os_error())
	} else {
		Ok(addr.cast::<Sym>().as_ref().unwrap_unchecked())
	}
}

#[inline]
pub(crate) unsafe fn dylib_close_and_exit(lib_handle: Handle, exit_code: i32) -> ! {
	c::FreeLibraryAndExitThread(lib_handle, exit_code as u32)
}

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
	#[inline]
	fn as_handle(&self) -> BorrowedHandle<'_> {
		unsafe { BorrowedHandle::borrow_raw(self.0) }
	}
}

pub trait LibraryExt: Sealed {
	fn path(&self) -> io::Result<path::PathBuf>;
}

impl LibraryExt for Library {
	fn path(&self) -> io::Result<path::PathBuf> {
		const MAX_PATH: usize = 260;
		let mut file_name = vec![0u16; MAX_PATH];
		loop {
			let _ = unsafe {
				c::GetModuleFileNameW(self.0, file_name.as_mut_ptr(), file_name.len() as c::DWORD)
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

pub trait SymExt: Sealed {
	// This is just weird, but strangely makes sense.
	fn library(self) -> io::Result<Library>;
}

impl SymExt for &Sym {
	fn library(self) -> io::Result<Library> {
		let mut handle = ptr::null_mut();
		let result = unsafe {
			c::GetModuleHandleExW(
				c::GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
				self as *const Sym as *const _,
				&mut handle,
			)
		};
		if result == 0 {
			Err(io::Error::last_os_error())
		} else {
			Ok(Library(handle))
		}
	}
}

// symbol handlers are single threaded so they are not Send or Sync
#[derive(Debug)]
pub struct SymbolHandler(c::HANDLE);
// symbol handlers can be sent across threads, but does not implement `Sync`
unsafe impl Send for SymbolHandler {}
