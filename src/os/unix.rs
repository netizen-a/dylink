#![allow(clippy::let_unit_value)]

use super::Handle;
use crate::sealed::Sealed;
use crate::Sym;
use std::os::unix::ffi::OsStrExt;
use std::{ffi, io, mem, ptr};

#[cfg(not(any(target_os = "linux", target_os = "macos", target_env = "gnu")))]
use std::sync;

mod c;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_env = "gnu")))]
#[inline]
fn dylib_guard<'a>() -> sync::LockResult<sync::MutexGuard<'a, ()>> {
	static LOCK: sync::Mutex<()> = sync::Mutex::new(());
	LOCK.lock()
}

#[cfg(any(target_os = "linux", target_os = "macos", target_env = "gnu"))]
#[inline(always)]
fn dylib_guard() {}

unsafe fn c_dlerror() -> Option<ffi::CString> {
	let raw = c::dlerror();
	if raw.is_null() {
		None
	} else {
		Some(ffi::CStr::from_ptr(raw).to_owned())
	}
}

#[inline]
pub(crate) unsafe fn dylib_open(path: &ffi::OsStr) -> io::Result<Handle> {
	let _lock = dylib_guard();
	let c_str = ffi::CString::new(path.as_bytes())?;
	let handle: *mut ffi::c_void = c::dlopen(c_str.as_ptr(), c::RTLD_NOW);
	if handle.is_null() {
		let err = c_dlerror().unwrap();
		Err(io::Error::new(io::ErrorKind::Other, err.to_str().unwrap()))
	} else {
		Ok(handle.cast())
	}
}

#[inline]
pub(crate) unsafe fn dylib_this() -> io::Result<Handle> {
	let _lock = dylib_guard();
	let handle: *mut ffi::c_void = c::dlopen(ptr::null(), c::RTLD_NOW);
	if handle.is_null() {
		let err = c_dlerror().unwrap();
		Err(io::Error::new(io::ErrorKind::Other, err.to_str().unwrap()))
	} else {
		Ok(handle.cast())
	}
}

#[inline]
pub(crate) unsafe fn dylib_close(lib_handle: Handle) -> io::Result<()> {
	let _lock = dylib_guard();
	if c::dlclose(lib_handle.cast()) != 0 {
		let err = c_dlerror().unwrap();
		Err(io::Error::new(io::ErrorKind::Other, err.to_str().unwrap()))
	} else {
		Ok(())
	}
}

#[inline]
pub(crate) unsafe fn dylib_symbol<'a>(lib_handle: Handle, name: &str) -> io::Result<&'a Sym> {
	let _lock = dylib_guard();
	let c_str = ffi::CString::new(name).unwrap();

	let _ = c::dlerror(); // clear existing errors
	let handle: *mut ffi::c_void = c::dlsym(lib_handle.cast(), c_str.as_ptr()).cast_mut();

	if let Some(err) = c_dlerror() {
		Err(io::Error::new(io::ErrorKind::Other, err.to_str().unwrap()))
	} else {
		Ok(handle.cast::<Sym>().as_ref().unwrap_unchecked())
	}
}

#[inline]
pub(crate) unsafe fn dylib_close_and_exit(lib_handle: Handle, exit_code: i32) -> ! {
	let _ = dylib_close(lib_handle);
	std::process::exit(exit_code)
}

// This function doesn't use a lock because we don't check errors.
#[cfg(any(target_os = "linux", target_os = "macos", target_env = "gnu"))]
#[inline]
pub(crate) unsafe fn dylib_is_loaded(path: &ffi::OsStr) -> bool {
	let c_str = ffi::CString::new(path.as_bytes()).expect("failed to create CString");
	let result = c::dlopen(c_str.as_ptr(), c::RTLD_NOW | c::RTLD_NOLOAD);
	if result.is_null() {
		false
	} else {
		let _ = c::dlclose(result);
		true
	}
}

#[derive(Debug)]
pub struct DlInfo {
	pub path: ffi::CString,
	pub base: *mut ffi::c_void,
	pub name: ffi::CString,
	pub addr: *const Sym,
}

pub trait SymExt: Sealed {
	fn info(&self) -> io::Result<DlInfo>;
}

impl SymExt for Sym {
	fn info<'a>(&self) -> io::Result<DlInfo> {
		let mut info = mem::MaybeUninit::<c::Dl_info>::zeroed();
		unsafe {
			if c::dladdr(self as *const Sym as *const _, info.as_mut_ptr()) != 0 {
				let info = info.assume_init();
				Ok(DlInfo {
					path: ffi::CStr::from_ptr(info.dli_fname).to_owned(),
					base: info.dli_fbase,
					name: ffi::CStr::from_ptr(info.dli_sname).to_owned(),
					addr: info.dli_saddr as *const Sym,
				})
			} else {
				let err = ffi::CStr::from_ptr(c::dlerror()).to_owned();
				Err(io::Error::new(io::ErrorKind::Other, err.to_str().unwrap()))
			}
		}
	}
}
