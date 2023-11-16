// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
use std::marker::PhantomData;
use std::os::windows::ffi::{OsStrExt, OsStringExt};

use std::os::windows::prelude::*;
use std::{ffi, io, mem, path, ptr};

use crate::weak;
use crate::{Library, Symbol};

mod c;

fn to_wide(path: &ffi::OsStr) -> Vec<u16> {
	path.encode_wide().chain(std::iter::once(0u16)).collect()
}

impl super::InnerLibrary {
	pub unsafe fn open(path: &ffi::OsStr) -> io::Result<Self> {
		let wide_str: Vec<u16> = to_wide(path);
		let handle = c::LoadLibraryExW(wide_str.as_ptr(), ptr::null_mut(), 0);
		ptr::NonNull::new(handle)
			.ok_or_else(io::Error::last_os_error)
			.map(Self)
	}

	pub unsafe fn this() -> io::Result<Self> {
		let mut handle: *mut ffi::c_void = ptr::null_mut();
		c::GetModuleHandleExW(0, ptr::null(), &mut handle);
		ptr::NonNull::new(handle)
			.ok_or_else(io::Error::last_os_error)
			.map(Self)
	}

	#[inline]
	pub unsafe fn c_symbol(&self, name: &ffi::CStr) -> *const ffi::c_void {
		c::GetProcAddress(self.0.as_ptr(), name.as_ptr())
	}

	pub unsafe fn symbol<'a>(&self, name: &str) -> io::Result<Symbol<'a>> {
		let c_str = ffi::CString::new(name).unwrap();
		let addr: *const ffi::c_void = self.c_symbol(&c_str);
		if addr.is_null() {
			Err(io::Error::last_os_error())
		} else {
			Ok(Symbol(addr.cast_mut(), PhantomData))
		}
	}

	pub(crate) unsafe fn path(&self) -> io::Result<path::PathBuf> {
		const MAX_PATH: usize = 260;
		const ERROR_INSUFFICIENT_BUFFER: i32 = 0x7A;

		let mut file_name = vec![0u16; MAX_PATH];
		loop {
			let _ = c::GetModuleFileNameW(
				self.0.as_ptr(),
				file_name.as_mut_ptr(),
				file_name.len() as c::DWORD,
			);
			let last_error = io::Error::last_os_error();
			match last_error.raw_os_error().unwrap_unchecked() {
				0 => {
					// The function succeeded.
					// Truncate the vector to remove unused zero bytes.
					if let Some(new_len) = file_name.iter().rposition(|&a| a != 0) {
						file_name.truncate(new_len + 1)
					}
					let os_str = ffi::OsString::from_wide(&file_name);
					break Ok(os_str.into());
				}
				ERROR_INSUFFICIENT_BUFFER => {
					// The buffer is too small; double its size.
					file_name.resize(file_name.len() * 2, 0)
				}
				_ => {
					// An unexpected error occurred; return an error.
					return Err(last_error);
				}
			}
		}
	}
	pub(crate) unsafe fn try_clone(&self) -> io::Result<Self> {
		let mut new_handle = ptr::null_mut();
		let _ = c::GetModuleHandleExW(
			c::GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
			self.0.as_ptr().cast(),
			&mut new_handle,
		);
		ptr::NonNull::new(new_handle)
			.ok_or_else(io::Error::last_os_error)
			.map(Self)
	}
	pub(crate) unsafe fn from_weak(addr: *mut super::Header) -> Option<Self> {
		if let Some(addr) = ptr::NonNull::new(addr.cast::<ffi::c_void>()) {
			let new_lib = super::InnerLibrary(addr);
			new_lib.try_clone().ok()
		} else {
			None
		}
	}

	#[inline]
	pub(crate) unsafe fn get_addr(&self) -> *const super::Header {
		self.0.as_ptr().cast()
	}
}

impl Drop for super::InnerLibrary {
	fn drop(&mut self) {
		unsafe {
			c::FreeLibrary(self.0.as_ptr());
		}
	}
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

pub(crate) unsafe fn base_addr(symbol: *mut std::ffi::c_void) -> io::Result<*mut super::Header> {
	let mut handle = ptr::null_mut();
	let result = c::GetModuleHandleExW(
		c::GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT | c::GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
		symbol.cast(),
		&mut handle,
	);
	if result == 0 {
		Err(io::Error::last_os_error())
	} else {
		// The handle doubles as the base address (this may not be true the other way around though).
		Ok(handle.cast())
	}
}

pub(crate) unsafe fn load_objects() -> io::Result<Vec<weak::Weak>> {
	const INITIAL_SIZE: usize = 1000;
	let process_handle = c::GetCurrentProcess();
	let mut module_handles = vec![ptr::null_mut::<super::Header>(); INITIAL_SIZE];
	let mut len_needed: u32 = 0;
	let mut prev_size = INITIAL_SIZE;

	loop {
		let cb = (module_handles.len() * mem::size_of::<c::HANDLE>()) as u32;
		let result = c::EnumProcessModulesEx(
			process_handle,
			module_handles.as_mut_ptr().cast(),
			cb,
			&mut len_needed,
			c::LIST_MODULES_ALL,
		);
		if result == 0 {
			return Err(io::Error::last_os_error());
		}
		len_needed /= mem::size_of::<c::HANDLE>() as u32;
		if len_needed as usize > module_handles.len() {
			// We can't trust the next iteration to be bigger, so fill with null
			module_handles.fill(ptr::null_mut());
			// make the new size sufficiently bigger, and always grow instead of shrink.
			let new_size: usize = (prev_size).max(len_needed as usize + 30);
			prev_size = new_size;
			module_handles.resize(new_size, ptr::null_mut());
		} else {
			// success, so truncate to the appropriate size
			if let Some(new_len) = module_handles.iter().rposition(|a| !a.is_null()) {
				module_handles.truncate(new_len)
			}
			let module_handles = module_handles
				.into_iter()
				.map(|base_addr| {
					let hmodule =
						super::InnerLibrary(ptr::NonNull::new_unchecked(base_addr.cast()));
					weak::Weak {
						base_addr,
						path_name: hmodule.path().ok(),
					}
				})
				.collect::<Vec<weak::Weak>>();
			// box and return the slice
			return Ok(module_handles);
		}
	}
}
