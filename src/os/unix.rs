#![allow(clippy::let_unit_value)]

use super::Handle;
#[cfg(any(feature = "unstable", docsrs))]
use crate::sealed::Sealed;
use crate::Symbol;
use std::marker::PhantomData;
use std::os::unix::ffi::OsStrExt;
use std::{ffi, io, mem, path, ptr};

#[cfg(not(any(target_os = "linux", target_env = "gnu")))]
use std::sync;

mod c;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_env = "gnu")))]
#[inline]
fn dylib_guard<'a>() -> sync::LockResult<sync::MutexGuard<'a, ()>> {
	static LOCK: sync::Mutex<()> = sync::Mutex::new(());
	LOCK.lock()
}

#[cfg(any(target_os = "linux", target_env = "gnu"))]
#[inline(always)]
fn dylib_guard() {}

#[cfg(target_os = "macos")]
static LOCK: sync::RwLock<()> = sync::RwLock::new(());

#[cfg(target_os = "macos")]
#[inline]
fn dylib_guard<'a>() -> sync::LockResult<sync::RwLockReadGuard<'a, ()>> {
	LOCK.read()
}

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
	let handle: *mut ffi::c_void = c::dlopen(c_str.as_ptr(), c::RTLD_NOW | c::RTLD_LOCAL);
	if handle.is_null() {
		let err = c_dlerror().unwrap();
		Err(io::Error::new(io::ErrorKind::Other, err.to_string_lossy()))
	} else {
		Ok(handle.cast())
	}
}

#[inline]
pub(crate) unsafe fn dylib_this() -> io::Result<Handle> {
	let _lock = dylib_guard();
	let handle: *mut ffi::c_void = c::dlopen(ptr::null(), c::RTLD_NOW | c::RTLD_LOCAL);
	if handle.is_null() {
		let err = c_dlerror().unwrap();
		Err(io::Error::new(io::ErrorKind::Other, err.to_string_lossy()))
	} else {
		Ok(handle.cast())
	}
}

#[inline]
pub(crate) unsafe fn dylib_close(lib_handle: Handle) -> io::Result<()> {
	let _lock = dylib_guard();
	if c::dlclose(lib_handle.cast()) != 0 {
		let err = c_dlerror().unwrap();
		Err(io::Error::new(io::ErrorKind::Other, err.to_string_lossy()))
	} else {
		Ok(())
	}
}

#[inline]
pub(crate) unsafe fn dylib_symbol<'a>(lib_handle: Handle, name: &str) -> io::Result<Symbol<'a>> {
	let _lock = dylib_guard();
	let c_str = ffi::CString::new(name).unwrap();

	let _ = c_dlerror(); // clear existing errors
	let handle: *mut ffi::c_void = c::dlsym(lib_handle.cast(), c_str.as_ptr()).cast_mut();

	if let Some(err) = c_dlerror() {
		Err(io::Error::new(io::ErrorKind::Other, err.to_string_lossy()))
	} else {
		Ok(Symbol(handle, PhantomData))
	}
}

#[inline]
pub(crate) unsafe fn dylib_close_and_exit(lib_handle: Handle, exit_code: i32) -> ! {
	let _ = dylib_close(lib_handle);
	std::process::exit(exit_code)
}

#[cfg(any(target_env="gnu", target_os="macos"))]
pub(crate) unsafe fn dylib_path(handle: Handle) -> io::Result<path::PathBuf> {
	use std::os::unix::ffi::OsStringExt;

	if let Ok(this_handle) = dylib_this() {
		dylib_close(this_handle)?;
		// This handles an edge case where link_map can't see the current executable.
		if (cfg!(target_os = "macos") && (this_handle as isize & (-4)) == (handle as isize & (-4)))
			|| this_handle == handle
		{
			return std::env::current_exe();
		}
	}

	#[cfg(target_env = "gnu")]
	{
		let mut map_ptr = ptr::null_mut::<c::link_map>();
		if c::dlinfo(
			handle as *mut _,
			c::RTLD_DI_LINKMAP,
			&mut map_ptr as *mut _ as *mut _,
		) == 0
		{
			let path = ffi::CStr::from_ptr((*map_ptr).l_name).to_owned();
			let path = ffi::OsString::from_vec(path.into_bytes());
			if path.len() > 0 {
				Ok(path::PathBuf::from(path))
			} else {
				Err(io::Error::new(io::ErrorKind::NotFound, "path not found"))
			}
		} else {
			let err = c_dlerror().unwrap();
			Err(io::Error::new(io::ErrorKind::Other, err.to_string_lossy()))
		}
	}
	#[cfg(target_os = "macos")]
	{
		let _guard = LOCK.write();
		for x in (0..c::_dyld_image_count()).rev() {
			let image_name = c::_dyld_get_image_name(x);
			// test if iterator is out of bounds.
			if image_name.is_null() {
				std::unreachable!("dylink encountered a potential race condition. Please submit an issue at `https://github.com/Razordor/dylink/issues`")
			}
			let active_handle = c::dlopen(image_name, c::RTLD_NOW | c::RTLD_LOCAL | c::RTLD_NOLOAD);
			if !active_handle.is_null() {
				let _ = c::dlclose(active_handle);
			}
			if (handle as isize & (-4)) == (active_handle as isize & (-4)) {
				let pathname = ffi::CStr::from_ptr(image_name).to_owned();
				let pathname = ffi::OsString::from_vec(pathname.into_bytes());
				return Ok(path::PathBuf::from(pathname));
			}
		}
		Err(io::Error::new(io::ErrorKind::NotFound, "path not found"))
	}
}

pub(crate) unsafe fn base_addr(symbol: &Symbol) -> io::Result<*const ffi::c_void> {
	let mut info = mem::MaybeUninit::<c::Dl_info>::zeroed();
	if c::dladdr(symbol.cast(), info.as_mut_ptr()) != 0 {
		let info = info.assume_init();
		Ok(info.dli_fbase)
	} else {
		// dlerror is not available for dladdr, so we're giving a generic error.
		Err(io::Error::new(
			io::ErrorKind::Other,
			"failed to get symbol info",
		))
	}
}

#[cfg(any(feature = "unstable", docsrs))]
#[derive(Debug)]
pub struct SymInfo<'a> {
	pub path: ffi::CString,
	pub base: *mut ffi::c_void,
	pub name: ffi::CString,
	pub addr: Symbol<'a>,
}

#[cfg(any(feature = "unstable", docsrs))]
pub trait SymExt: Sealed {
	fn info(&self) -> io::Result<SymInfo>;
}

#[cfg(any(feature = "unstable", docsrs))]
impl SymExt for Symbol<'_> {
	#[doc(alias = "dladdr")]
	fn info(&self) -> io::Result<SymInfo> {
		let mut info = mem::MaybeUninit::<c::Dl_info>::zeroed();
		unsafe {
			if c::dladdr(self.0 as *const _, info.as_mut_ptr()) != 0 {
				let info = info.assume_init();
				Ok(SymInfo {
					path: ffi::CStr::from_ptr(info.dli_fname).to_owned(),
					base: info.dli_fbase,
					name: ffi::CStr::from_ptr(info.dli_sname).to_owned(),
					addr: Symbol(info.dli_saddr, PhantomData),
				})
			} else {
				Err(io::Error::new(
					io::ErrorKind::Other,
					"failed to get symbol info",
				))
			}
		}
	}
}


