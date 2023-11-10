// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
#![allow(clippy::let_unit_value)]
#![allow(unused_imports)]

#[cfg(target_env="gnu")]
use libc::dl_iterate_phdr;

use super::Handle;
use crate::sealed::Sealed;
use crate::{Symbol, weak};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::{ffi, io, mem, path::PathBuf, ptr};
use std::{
	marker::PhantomData,
	sync::{
		atomic::{AtomicU32, Ordering},
		Once,
	},
};

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
	let raw = libc::dlerror();
	if raw.is_null() {
		None
	} else {
		Some(ffi::CStr::from_ptr(raw).to_owned())
	}
}

pub(crate) unsafe fn dylib_open(path: &ffi::OsStr) -> io::Result<Handle> {
	let _lock = dylib_guard();
	let c_str = ffi::CString::new(path.as_bytes())?;
	let handle: *mut ffi::c_void = libc::dlopen(c_str.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL);
	if let Some(ret) = ptr::NonNull::new(handle) {
		Ok(ret)
	} else {
		let err = c_dlerror().unwrap();
		Err(io::Error::new(io::ErrorKind::Other, err.to_string_lossy()))
	}
}

pub(crate) unsafe fn dylib_this() -> io::Result<Handle> {
	let _lock = dylib_guard();
	let handle: *mut ffi::c_void = libc::dlopen(ptr::null(), libc::RTLD_NOW | libc::RTLD_LOCAL);
	if let Some(ret) = ptr::NonNull::new(handle) {
		Ok(ret)
	} else {
		let err = c_dlerror().unwrap();
		Err(io::Error::new(io::ErrorKind::Other, err.to_string_lossy()))
	}
}

pub(crate) unsafe fn dylib_close(lib_handle: Handle) -> io::Result<()> {
	let _lock = dylib_guard();
	if libc::dlclose(lib_handle.as_ptr()) != 0 {
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
	let handle: *mut ffi::c_void = libc::dlsym(lib_handle, c_str.as_ptr());

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
	if libc::dlinfo(
		handle.as_ptr(),
		libc::RTLD_DI_LINKMAP,
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

	let mut result = Err(io::Error::new(io::ErrorKind::NotFound, "Path not found"));
	let _ = get_image_count().fetch_update(Ordering::SeqCst, Ordering::SeqCst, |image_index| {
		for image_index in (0..image_index).rev() {
			let image_name = c::_dyld_get_image_name(image_index);
			let active_handle = libc::dlopen(image_name, libc::RTLD_NOW | libc::RTLD_LOCAL | libc::RTLD_NOLOAD);
			if !active_handle.is_null() {
				let _ = libc::dlclose(active_handle);
			}
			if (handle.as_ptr() as isize & (-4)) == (active_handle as isize & (-4)) {
				let path = ffi::CStr::from_ptr(image_name);
				let path = ffi::OsStr::from_bytes(path.to_bytes());
				result = Ok(path.into());
				break;
			}
		}
		Some(image_index)
	});
	result
}

pub(crate) unsafe fn base_addr(symbol: *mut std::ffi::c_void) -> io::Result<*mut ffi::c_void> {
	let mut info = mem::MaybeUninit::<libc::Dl_info>::zeroed();
	if libc::dladdr(symbol, info.as_mut_ptr()) != 0 {
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

#[cfg(target_env="gnu")]
unsafe fn iter_phdr<F>(mut f: F) -> ffi::c_int
where F: FnMut(*mut libc::dl_phdr_info, libc::size_t) -> ffi::c_int
{
	unsafe extern "C" fn callback<F>(info: *mut libc::dl_phdr_info, size: libc::size_t, data: *mut ffi::c_void) -> ffi::c_int
	where F: FnMut(*mut libc::dl_phdr_info, libc::size_t) -> ffi::c_int
	{
		let f = data as *mut F;
		(*f)(info, size)
	}
	libc::dl_iterate_phdr(Some(callback::<F>), &mut f as *mut _ as *mut _)
}

#[cfg(target_env = "gnu")]
pub(crate) unsafe fn load_objects() -> io::Result<Vec<weak::Weak>> {
	let mut data = Vec::new();
	let _ = iter_phdr(|info, _|{

		let path_name = if (*info).dlpi_name.is_null() {
			None
		} else {
			let path = ffi::CStr::from_ptr((*info).dlpi_name);
			let path = ffi::OsStr::from_bytes(path.to_bytes());
			Some(PathBuf::from(path))
		};
		let weak_ptr = weak::Weak{
			base_addr: (*info).dlpi_addr as *mut ffi::c_void,
			path_name,
		};
		data.push(weak_ptr);
		0
	});
	Ok(data)
}

#[cfg(target_os = "macos")]
pub(crate) unsafe fn load_objects() -> io::Result<Vec<weak::Weak>> {
	use std::os::unix::ffi::OsStringExt;

	let mut data = Vec::new();
	let _ = get_image_count().fetch_update(Ordering::SeqCst, Ordering::SeqCst, |image_index| {
		data.clear();
		for image_index in 0..image_index {
			let path = ffi::CStr::from_ptr(c::_dyld_get_image_name(image_index));
			let path = ffi::OsStr::from_bytes(path.to_bytes());
			let weak_ptr = weak::Weak{
				base_addr: c::_dyld_get_image_header(image_index) as *mut ffi::c_void,
				path_name: PathBuf::from(path)
			};
			data.push(weak_ptr);
		}
		Some(image_index)
	});
	Ok(data)
}

pub(crate) unsafe fn dylib_upgrade(addr: *mut ffi::c_void) -> Option<Handle> {
	let mut info = mem::MaybeUninit::zeroed();
	if libc::dladdr(addr, info.as_mut_ptr()) != 0 {
		let info = info.assume_init();
		let handle = libc::dlopen(info.dli_fname, libc::RTLD_NOW | libc::RTLD_LOCAL);
		Handle::new(handle)
	} else {
		None
	}
}

// returns null if handle is invalid
#[cfg(target_env = "gnu")]
pub(crate) unsafe fn get_addr(handle: Handle) -> *mut ffi::c_void {
	use std::os::unix::ffi::OsStringExt;
	let mut map_ptr = ptr::null_mut::<c::link_map>();
	if libc::dlinfo(
		handle.as_ptr(),
		libc::RTLD_DI_LINKMAP,
		&mut map_ptr as *mut _ as *mut _,
	) == 0
	{
		(*map_ptr).l_addr as *mut ffi::c_void
	} else {
		ptr::null_mut()
	}
}

// returns null if handle is invalid
#[cfg(target_os = "macos")]
pub(crate) unsafe fn get_addr(handle: Handle) -> *mut ffi::c_void {
	use std::os::unix::ffi::OsStringExt;

	let mut result = ptr::null_mut();
	let _ = get_image_count().fetch_update(Ordering::SeqCst, Ordering::SeqCst, |image_index| {
		for image_index in (0..image_index).rev() {
			let image_name = c::_dyld_get_image_name(image_index);
			let active_handle = libc::dlopen(image_name, libc::RTLD_NOW | libc::RTLD_LOCAL | libc::RTLD_NOLOAD);
			if !active_handle.is_null() {
				let _ = libc::dlclose(active_handle);
			}
			if (handle.as_ptr() as isize & (-4)) == (active_handle as isize & (-4)) {
				result = c::_dyld_get_image_header(image_index) as *mut ffi::c_void;
				break;
			}
		}
		Some(image_index)
	});
	result
}