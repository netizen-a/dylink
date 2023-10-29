// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
#![allow(clippy::let_unit_value)]
#![allow(unused_imports)]

use super::Handle;
use crate::sealed::Sealed;
use crate::Symbol;
use std::{marker::PhantomData, sync::{atomic::{Ordering, AtomicU32}, Once}};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::{ffi, io, mem, path::PathBuf, ptr};

#[cfg(not(any(target_os = "linux", target_env = "gnu")))]
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

pub(crate) unsafe fn dylib_open(path: &ffi::OsStr) -> io::Result<Handle> {
	let _lock = dylib_guard();
	let c_str = ffi::CString::new(path.as_bytes())?;
	let handle: *mut ffi::c_void = c::dlopen(c_str.as_ptr(), c::RTLD_NOW | c::RTLD_LOCAL);
	if let Some(ret) = ptr::NonNull::new(handle) {
		Ok(ret)
	} else {
		let err = c_dlerror().unwrap();
		Err(io::Error::new(io::ErrorKind::Other, err.to_string_lossy()))
	}
}

pub(crate) unsafe fn dylib_this() -> io::Result<Handle> {
	let _lock = dylib_guard();
	let handle: *mut ffi::c_void = c::dlopen(ptr::null(), c::RTLD_NOW | c::RTLD_LOCAL);
	if let Some(ret) = ptr::NonNull::new(handle) {
		Ok(ret)
	} else {
		let err = c_dlerror().unwrap();
		Err(io::Error::new(io::ErrorKind::Other, err.to_string_lossy()))
	}
}

pub(crate) unsafe fn dylib_close(lib_handle: Handle) -> io::Result<()> {
	let _lock = dylib_guard();
	if c::dlclose(lib_handle.as_ptr()) != 0 {
		let err = c_dlerror().unwrap();
		Err(io::Error::new(io::ErrorKind::Other, err.to_string_lossy()))
	} else {
		Ok(())
	}
}

pub(crate) unsafe fn dylib_symbol<'a>(
	lib_handle: *mut ffi::c_void,
	name: &str,
) -> io::Result<Symbol<'a>> {
	let _lock = dylib_guard();
	let c_str = ffi::CString::new(name).unwrap();

	let _ = c_dlerror(); // clear existing errors
	let handle: *mut ffi::c_void = c::dlsym(lib_handle, c_str.as_ptr()).cast_mut();

	if let Some(err) = c_dlerror() {
		Err(io::Error::new(io::ErrorKind::Other, err.to_string_lossy()))
	} else {
		Ok(Symbol(handle, PhantomData))
	}
}

pub(crate) unsafe fn dylib_path(handle: Handle) -> io::Result<PathBuf> {
	match dylib_this() {
		Ok(this_handle)
			if (cfg!(target_os = "macos")
				&& (this_handle.as_ptr() as isize & (-4)) == (handle.as_ptr() as isize & (-4)))
				|| this_handle == handle =>
		{
			std::env::current_exe()
		}
		_ => {
			#[cfg(target_env = "gnu")]
			{
				if let Some(path) = get_link_map_path(handle) {
					Ok(path)
				} else {
					Err(io::Error::new(
						io::ErrorKind::NotFound,
						"Library path not found",
					))
				}
			}
			#[cfg(target_os = "macos")]
			{
				get_macos_image_path(handle)
			}
			#[cfg(not(any(target_env = "gnu", target_os = "macos")))]
			{
				// Handle other platforms or configurations
				Err(io::Error::new(io::ErrorKind::Other, "Unsupported platform"))
			}
		}
	}
}

#[cfg(target_env = "gnu")]
unsafe fn get_link_map_path(handle: Handle) -> Option<PathBuf> {
	use std::os::unix::ffi::OsStringExt;
	let mut map_ptr = ptr::null_mut::<c::link_map>();
	if c::dlinfo(
		handle.as_ptr(),
		c::RTLD_DI_LINKMAP,
		&mut map_ptr as *mut _ as *mut _,
	) == 0
	{
		let path = ffi::CStr::from_ptr((*map_ptr).l_name);
		let path = ffi::OsStr::from_bytes(path.to_bytes());
		if !path.is_empty() {
			Some(path.into())
		} else {
			None
		}
	} else {
		None
	}
}


#[cfg(target_os = "macos")]
fn get_image_count() -> &'static AtomicU32 {
	static IMAGE_COUNT: AtomicU32 = AtomicU32::new(0);
	static START: Once = Once::new();
	extern "C" fn increment_count(_: *const c::mach_header, _: isize) {
		IMAGE_COUNT.fetch_add(1, Ordering::SeqCst);
	}
	extern "C" fn decrement_count(_: *const c::mach_header, _: isize) {
		IMAGE_COUNT.fetch_sub(1, Ordering::SeqCst);
	}
	START.call_once(|| unsafe {
		c::_dyld_register_func_for_add_image(increment_count);
		c::_dyld_register_func_for_remove_image(decrement_count);
	});

	&IMAGE_COUNT
}

#[cfg(target_os = "macos")]
unsafe fn get_macos_image_path(handle: Handle) -> io::Result<PathBuf> {
	use std::os::unix::ffi::OsStringExt;

	let count: &AtomicU32 = get_image_count();
	let mut result = Err(io::Error::new(io::ErrorKind::NotFound, "Path not found"));
	let _ = count.fetch_update(
		Ordering::SeqCst,
		Ordering::SeqCst,
		|i|{
			for j in (0..i).rev() {
				let image_name = c::_dyld_get_image_name(j);
				let active_handle = c::dlopen(image_name, c::RTLD_NOW | c::RTLD_LOCAL | c::RTLD_NOLOAD);
				if !active_handle.is_null() {
					let _ = c::dlclose(active_handle);
				}
				if (handle.as_ptr() as isize & (-4)) == (active_handle as isize & (-4)) {
					let path = ffi::CStr::from_ptr(image_name);
					let path = ffi::OsStr::from_bytes(path.to_bytes());
					result = Ok(path.into());
					break;
				}
			}
			Some(i)
		});
	result
}

pub(crate) unsafe fn base_addr(symbol: *mut std::ffi::c_void) -> io::Result<*mut ffi::c_void> {
	let mut info = mem::MaybeUninit::<c::Dl_info>::zeroed();
	if c::dladdr(symbol, info.as_mut_ptr()) != 0 {
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

pub(crate) unsafe fn dylib_clone(handle: Handle) -> io::Result<Handle> {
	let this = dylib_this()?;
	if this == handle {
		Ok(this)
	} else {
		dylib_close(this)?;
		let path = dylib_path(handle)?;
		dylib_open(path.as_os_str())
	}
}

#[cfg(feature = "unstable")]
#[derive(Debug)]
pub struct DlInfo {
	pub dli_fname: ffi::CString,
	pub dli_fbase: *mut ffi::c_void,
	pub dli_sname: ffi::CString,
	pub dli_saddr: *mut ffi::c_void,
}

#[cfg(feature = "unstable")]
pub trait SymExt: Sealed {
	fn info(&self) -> io::Result<DlInfo>;
}

#[cfg(feature = "unstable")]
impl SymExt for Symbol<'_> {
	#[doc(alias = "dladdr")]
	fn info(&self) -> io::Result<DlInfo> {
		let mut info = mem::MaybeUninit::<c::Dl_info>::zeroed();
		unsafe {
			if c::dladdr(self.0 as *const _, info.as_mut_ptr()) != 0 {
				let info = info.assume_init();
				Ok(DlInfo {
					dli_fname: ffi::CStr::from_ptr(info.dli_fname).to_owned(),
					dli_fbase: info.dli_fbase,
					dli_sname: ffi::CStr::from_ptr(info.dli_sname).to_owned(),
					dli_saddr: info.dli_saddr,
				})
			} else {
				// dlerror isn't available for dlinfo, so I can only provide a general error message here
				Err(io::Error::new(
					io::ErrorKind::Other,
					"Failed to retrieve symbol information",
				))
			}
		}
	}
}
