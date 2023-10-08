use super::Handle;
use crate::Sym;
use std::os::unix::ffi::OsStrExt;
use std::{ffi, io, ptr};

#[cfg(not(any(linux, macos, target_env = "gnu")))]
use std::sync;

mod c;

#[cfg(not(any(linux, macos, target_env = "gnu")))]
#[inline]
fn dylib_guard<'a>() -> sync::LockResult<sync::MutexGuard<'a, ()>> {
	static LOCK: sync::Mutex<()> = sync::Mutex::new(());
	LOCK.lock()
}

#[cfg(any(linux, macos, target_env = "gnu"))]
#[inline(always)]
fn dylib_guard() {}

unsafe fn dylib_error() -> io::Result<()> {
	let raw = c::dlerror();
	if raw.is_null() {
		Ok(())
	} else {
		let e = ffi::CStr::from_ptr(raw).to_owned();
		Err(io::Error::new(io::ErrorKind::Other, e.to_str().unwrap()))
	}
}

unsafe fn map_result<F>(f: F) -> io::Result<*mut ffi::c_void>
where
	F: FnOnce() -> *mut ffi::c_void,
{
	let _lock = dylib_guard();
	let _ = c::dlerror(); // clear existing errors
	let handle: *mut ffi::c_void = f();
	dylib_error().and(Ok(handle))
}

#[inline]
pub(crate) unsafe fn dylib_open(path: &ffi::OsStr) -> io::Result<Handle> {
	let c_str = ffi::CString::new(path.as_bytes())?;
	map_result(|| c::dlopen(c_str.as_ptr(), c::RTLD_NOW))
}

#[inline]
pub(crate) unsafe fn dylib_this() -> io::Result<Handle> {
	map_result(|| c::dlopen(ptr::null(), c::RTLD_NOW))
}

#[inline]
pub(crate) unsafe fn dylib_close(lib_handle: Handle) -> io::Result<()> {
	let _lock = dylib_guard();
	let _ = c::dlerror(); // clear existing errors
	if c::dlclose(lib_handle) != 0 {
		dylib_error()
	} else {
		Ok(())
	}
}

#[inline]
pub(crate) unsafe fn dylib_symbol<'a>(lib_handle: Handle, name: &str) -> io::Result<&'a Sym> {
	let c_str = ffi::CString::new(name).unwrap();
	map_result(|| c::dlsym(lib_handle, c_str.as_ptr()).cast_mut())
		.map(|p| p.cast::<Sym>().as_ref().unwrap_unchecked())
}

#[inline]
pub(crate) unsafe fn dylib_close_and_exit(lib_handle: Handle, exit_code: i32) -> ! {
	let _ = dylib_close(lib_handle);
	std::process::exit(exit_code)
}

// This function doesn't use a lock because we don't check errors.
#[cfg(any(linux, macos, target_env = "gnu"))]
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
