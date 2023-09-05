// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use super::*;
use std::{
	ffi,
	io,
	sync::atomic::Ordering,
};
#[cfg(windows)]
use std::os::windows::prelude::{IntoRawHandle, RawHandle};


unsafe impl Loader for System {
	/// If successful, increments reference count to shared library handle, and constructs `SystemLoader`.
	unsafe fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
		let handle: *mut ffi::c_void;
		let path = path.as_ref();
		#[cfg(unix)]
		{
			use crate::os::unix::*;
			let c_str = if let Some(val) = path.as_os_str().to_str() {
				ffi::CString::new(val)
			} else {
				//FIXME: change to use a stricter error when stable.
				//return Err(io::ErrorKind::InvalidFilename.into())
				ffi::CString::new("")
			}?;
			handle = dlopen(c_str.as_ptr(), RTLD_NOW | RTLD_LOCAL);
		}
		#[cfg(windows)]
		{
			use crate::os::win32::*;
			use std::os::windows::ffi::OsStrExt;
			let os_str = path.as_os_str();
			let wide_str: Vec<u16> = os_str.encode_wide().chain(std::iter::once(0u16)).collect();
			handle = crate::os::win32::LoadLibraryExW(
				wide_str.as_ptr().cast(),
				std::ptr::null_mut(),
				LOAD_LIBRARY_SEARCH_DEFAULT_DIRS | LOAD_LIBRARY_SAFE_CURRENT_DIRS,
			);
		}
		if handle.is_null() {
			Err(io::Error::last_os_error())
		} else {
			Ok(Self(handle.into()))
		}
	}

	unsafe fn sym(&self, symbol: &str) -> *const () {
		let c_str = ffi::CString::new(symbol).unwrap();
		crate::os::dlsym(self.0.load(Ordering::Relaxed), c_str.as_ptr().cast())
	}
}

impl System {
	/// Decrements reference counter to shared library. When reference counter hits zero the library is unloaded.
	/// # Errors
	/// May error depending on system call.
	pub unsafe fn close(self) -> io::Result<()> {
		let result = crate::os::dlclose(self.0.into_inner());
		if (cfg!(windows) && result == 0) || (cfg!(unix) && result != 0) {
			Err(io::Error::last_os_error())
		} else {
			Ok(())
		}
	}
}

// rust's std doesn't have a unix equivalent trait for IntoRawHandle

#[cfg(windows)]
impl IntoRawHandle for System {
	fn into_raw_handle(self) -> RawHandle {
		self.0.into_inner()
	}
}