use std::{ffi, ptr, io};
use crate::Sym;

pub const RTLD_NOW: ffi::c_int = 0x2;
//#[cfg(target_env = "gnu")]
//pub const RTLD_DEFAULT: *mut ffi::c_void = ptr::null_mut();
extern "C" {
	pub fn dlopen(filename: *const ffi::c_char, flag: ffi::c_int) -> *mut ffi::c_void;
	pub fn dlerror() -> *const ffi::c_char;
    pub fn dlsym(handle: *mut ffi::c_void, symbol: *const ffi::c_char) -> *const ffi::c_void;
    pub fn dlclose(hlibmodule: *mut ffi::c_void) -> ffi::c_int;
}

pub unsafe fn dylib_open<P: AsRef<ffi::OsStr>>(path: P) -> io::Result<*mut ffi::c_void> {
    let handle: *mut ffi::c_void;
	let c_str = if let Some(val) = path.as_ref().to_str() {
		ffi::CString::new(val)
	} else {
		//FIXME: change to use io_error_more content error when stable.
		//return Err(io::ErrorKind::InvalidFilename.into())
		return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid filename"))
	}?;
	handle = dlopen(c_str.as_ptr(), RTLD_NOW);
	if handle.is_null() {
		// FIXME: This is not MT-safe on all platforms.
		let e = ffi::CStr::from_ptr(dlerror()).to_owned();
		Err(io::Error::new(io::ErrorKind::Other, e.to_str().unwrap()))
	} else {
		Ok(handle)
	}
}


pub unsafe fn dylib_this() -> io::Result<*mut ffi::c_void> {
	let handle: *mut ffi::c_void = dlopen(ptr::null(), RTLD_NOW);
	if handle.is_null() {
		// FIXME: This is not MT-safe on all platforms.
		let e = ffi::CStr::from_ptr(dlerror()).to_owned();
		Err(io::Error::new(io::ErrorKind::Other, e.to_str().unwrap()))
	} else {
		Ok(handle)
	}
}

pub unsafe fn dylib_close(lib_handle: *mut ffi::c_void) -> io::Result<()> {
	let result = dlclose(lib_handle);
    if result != 0 {
		// FIXME: This is not MT-safe on all platforms.
		let e = ffi::CStr::from_ptr(dlerror()).to_owned();
		Err(io::Error::new(io::ErrorKind::Other, e.to_str().unwrap()))
	} else {
		Ok(())
	}
}
pub unsafe fn dylib_symbol(lib_handle: *mut ffi::c_void, name: &str) -> io::Result<&Sym> {
    let c_str = ffi::CString::new(name).unwrap();
	let addr: *const () = unsafe {
		dlsym(lib_handle, c_str.as_ptr().cast()).cast()
	};
	if addr.is_null() {
		// FIXME: This is not MT-safe on all platforms.
		let e = ffi::CStr::from_ptr(dlerror()).to_owned();
		Err(io::Error::new(io::ErrorKind::Other, e.to_str().unwrap()))
	} else {
		Ok(addr.cast::<Sym>().as_ref().unwrap_unchecked())
	}
}