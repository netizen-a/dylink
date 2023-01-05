use std::{ffi, mem, sync::RwLock};

use crate::{error::*, FnPtr, Result, VkInstance};
// dylink_macro internally uses dylink as it's root namespace,
// but since we are in dylink the namespace is actually named `self`.
// this is just here to resolve the missing namespace issue.
extern crate self as dylink;

// the dylink macro is ironically used to resolve itself, but it only uses
// normal linking, so it's nonrecursive.
//
// windows and linux are fully tested and useable as of this comment.
// macos should theoretically work, but it's untested.
#[cfg_attr(windows, dylink_macro::dylink(name = "vulkan-1.dll"))]
#[cfg_attr(
	all(unix, not(target_os = "macos")),
	dylink_macro::dylink(any(name = "libvulkan.so.1", name = "libvulkan.so"))
)]
#[cfg_attr(
	target_os = "macos",
	dylink_macro::dylink(any(
		name = "libvulkan.dylib",
		name = "libvulkan.1.dylib",
		name = "libMoltenVK.dylib"
	))
)]
extern "system" {
	pub(crate) fn vkGetInstanceProcAddr(
		instance: VkInstance,
		pName: *const ffi::c_char,
	) -> Option<FnPtr>;
}

/// `vkloader` is a vulkan loader specialization.
/// If `instance` is null, then `device` is ignored.
#[cfg(any(windows, target_os = "linux"))]
pub unsafe fn vkloader(instance: Option<&VkInstance>, name: &'static ffi::CStr) -> Result<FnPtr> {
	let inst = instance.map_or(VkInstance(std::ptr::null()), |r| r.clone());
	match vkGetInstanceProcAddr(inst, name.as_ptr()) {
		Some(addr) => Ok(addr),
		None => Err(DylinkError::new(
			Some(name.to_str().unwrap()),
			ErrorKind::FnNotFound,
		)),
	}
}

/// `glloader` is an opengl loader specialization.
#[cfg(any(windows, target_os = "linux"))]
pub unsafe fn glloader(name: &'static ffi::CStr) -> Result<FnPtr> {
	let maybe_fn = {
		#[cfg(unix)]
		{
			#[dylink_macro::dylink(name = "opengl32")]
			extern "system" {
				pub(crate) fn glXGetProcAddress(pName: *const ffi::c_char) -> Option<FnPtr>;
			}
			glXGetProcAddress(name.as_ptr() as *const _)
		}
		#[cfg(windows)]
		{
			use windows_sys::Win32::Graphics::OpenGL::wglGetProcAddress;
			wglGetProcAddress(name.as_ptr() as *const _)
		}
	};
	match maybe_fn {
		Some(addr) => Ok(addr),
		None => Err(DylinkError::new(
			Some(name.to_str().unwrap()),
			ErrorKind::FnNotFound,
		)),
	}
}

/// `loader` is a generalization for all other dlls.
#[cfg(any(windows, unix))]
pub fn loader(lib_name: &'static ffi::CStr, fn_name: &'static ffi::CStr) -> Result<FnPtr> {
	use std::collections::HashMap;

	use once_cell::sync::Lazy;
	#[cfg(windows)]
	use windows_sys::Win32::System::LibraryLoader::{
		GetProcAddress, LoadLibraryExA, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
	};

	static DLL_DATA: RwLock<Lazy<HashMap<ffi::CString, isize>>> =
		RwLock::new(Lazy::new(HashMap::default));

	let read_lock = DLL_DATA.read().unwrap();

	let handle: isize = if let Some(handle) = read_lock.get(lib_name) {
		*handle
	} else {
		mem::drop(read_lock);

		let lib_handle = unsafe {
			#[cfg(windows)]
			{
				#[cfg(windows)]
				LoadLibraryExA(
					lib_name.as_ptr() as *const _,
					0,
					LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
				)
			}
			#[cfg(unix)]
			{
				libc::dlopen(lib_name.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) as isize
			}
		};
		if lib_handle == 0 {
			return Err(DylinkError::new(
				Some(lib_name.to_str().unwrap()),
				ErrorKind::LibNotFound,
			));
		} else {
			DLL_DATA
				.write()
				.unwrap()
				.insert(lib_name.to_owned(), lib_handle);
		}
		lib_handle
	};

	let maybe_fn: Option<FnPtr> = unsafe {
		#[cfg(windows)]
		{
			GetProcAddress(handle, fn_name.as_ptr() as *const _)
		}
		#[cfg(unix)]
		{
			let addr: *const libc::c_void =
				libc::dlsym(handle as *mut libc::c_void, fn_name.as_ptr());
			std::mem::transmute(addr)
		}
	};
	match maybe_fn {
		Some(addr) => Ok(addr),
		None => Err(DylinkError::new(
			Some(fn_name.to_str().unwrap()),
			ErrorKind::FnNotFound,
		)),
	}
}
