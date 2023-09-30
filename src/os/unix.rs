use crate::Sym;
use std::{ffi, io, ptr};

#[cfg(not(any(linux, macos, target_env = "gnu")))]
use std::sync;

pub const RTLD_NOW: ffi::c_int = 0x2;
//#[cfg(target_env = "gnu")]
//pub const RTLD_DEFAULT: *mut ffi::c_void = ptr::null_mut();
extern "C" {
	fn dlopen(filename: *const ffi::c_char, flag: ffi::c_int) -> *mut ffi::c_void;
	fn dlerror() -> *const ffi::c_char;
	fn dlsym(handle: *mut ffi::c_void, symbol: *const ffi::c_char) -> *const ffi::c_void;
	fn dlclose(hlibmodule: *mut ffi::c_void) -> ffi::c_int;
}

#[cfg(not(any(linux, macos, target_env = "gnu")))]
#[inline]
fn dylib_guard<'a>() -> sync::LockResult<sync::MutexGuard<'a, ()>> {
	static LOCK: sync::Mutex<()> = sync::Mutex::new(());
	LOCK.lock()
}

#[cfg(any(linux, macos, target_env = "gnu"))]
#[inline(always)]
fn dylib_guard() {}

unsafe fn dylib_error() -> io::Error {
	let e = ffi::CStr::from_ptr(dlerror()).to_owned();
	io::Error::new(io::ErrorKind::Other, e.to_str().unwrap())
}

pub(crate) unsafe fn dylib_open<P: AsRef<ffi::OsStr>>(path: P) -> io::Result<*mut ffi::c_void> {
	let _lock = dylib_guard();
	let _ = dlerror(); // clear existing errors
	let handle: *mut ffi::c_void;
	let c_str = if let Some(val) = path.as_ref().to_str() {
		ffi::CString::new(val)
	} else {
		//FIXME: change to use io_error_more content error when stable.
		//return Err(io::ErrorKind::InvalidFilename.into())
		return Err(io::Error::new(
			io::ErrorKind::InvalidData,
			"invalid filename",
		));
	}?;
	handle = dlopen(c_str.as_ptr(), RTLD_NOW);
	if handle.is_null() {
		Err(dylib_error())
	} else {
		Ok(handle)
	}
}

pub(crate) unsafe fn dylib_this() -> io::Result<*mut ffi::c_void> {
	let _lock = dylib_guard();
	let _ = dlerror(); // clear existing errors
	let handle: *mut ffi::c_void = dlopen(ptr::null(), RTLD_NOW);
	if handle.is_null() {
		Err(dylib_error())
	} else {
		Ok(handle)
	}
}

pub(crate) unsafe fn dylib_close(lib_handle: *mut ffi::c_void) -> io::Result<()> {
	let _lock = dylib_guard();
	let _ = dlerror(); // clear existing errors
	let result = dlclose(lib_handle);
	if result != 0 {
		Err(dylib_error())
	} else {
		Ok(())
	}
}
pub(crate) unsafe fn dylib_symbol(lib_handle: *mut ffi::c_void, name: &str) -> io::Result<&Sym> {
	let _lock = dylib_guard();
	let _ = dlerror(); // clear existing errors
	let c_str = ffi::CString::new(name).unwrap();
	let addr: *const () = unsafe { dlsym(lib_handle, c_str.as_ptr().cast()).cast() };
	if addr.is_null() {
		Err(dylib_error())
	} else {
		Ok(addr.cast::<Sym>().as_ref().unwrap_unchecked())
	}
}

pub(crate) unsafe fn dylib_close_and_exit(lib_handle: *mut ffi::c_void, exit_code: u32) -> ! {
	let _ = dylib_close(lib_handle);
	std::process::exit(exit_code)
}
