use std::{
	ffi, mem,
	sync::{atomic::Ordering, RwLock},
};

use once_cell::sync::Lazy;

use crate::{example::*, lazyfn::*, VK_CONTEXT, error::*};



/// `vkloader` is a vulkan loader specialization.
/// # Panics
/// This function might panic if `vulkan-1.dll` is not found or if the function is not found.
#[track_caller]
pub unsafe fn vkloader(fn_name: &str) -> Result<fn()> {
	let device = VK_CONTEXT.device.load(Ordering::Acquire);
	let instance = VK_CONTEXT.instance.load(Ordering::Acquire);
	let c_fn_name = ffi::CString::new(fn_name).unwrap();
	let maybe_fn = if let Some(device) = std::ptr::NonNull::new(device) {
		vkGetDeviceProcAddr(device, c_fn_name.as_ptr())
			.or_else(|| vkGetInstanceProcAddr(instance, c_fn_name.as_ptr()))
	} else {
		vkGetInstanceProcAddr(instance, c_fn_name.as_ptr())
	};
	match maybe_fn {
		Some(addr) => Ok(mem::transmute(addr)),
		None => Err(DylinkError::new(fn_name.to_owned(), ErrorKind::FnNotFound)),
	}
}

/// `glloader` is an opengl loader specialization.
/// # Panics
/// This function might panic if the function is not found.
#[track_caller]
pub unsafe fn glloader(fn_name: &str) -> Result<fn()> {
	use windows_sys::Win32::Graphics::OpenGL::wglGetProcAddress;
	let c_fn_name = ffi::CString::new(fn_name).unwrap();
	let maybe_fn = wglGetProcAddress(c_fn_name.as_ptr() as *const _);
	match maybe_fn {
		Some(addr) => Ok(mem::transmute(addr)),
		None => Err(DylinkError::new(fn_name.to_owned(), ErrorKind::FnNotFound)),
	}	
}

/// `loader` is a generalization for all other dlls.
/// # Panics
/// This function might panic if the `lib_name` dll is not found or if the function is not found.
#[track_caller]
pub unsafe fn loader(lib_name: &str, fn_name: &str) -> Result<fn()> {
	use std::collections::HashMap;

	use windows_sys::Win32::{
		Foundation::HINSTANCE,
		System::LibraryLoader::{GetProcAddress, LoadLibraryExA, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS},
	};

	static DLL_DATA: Lazy<RwLock<HashMap<String, HINSTANCE>>> =
		Lazy::new(|| RwLock::new(HashMap::new()));

	let read_lock = DLL_DATA.read().unwrap();
	let handle: HINSTANCE = match read_lock.get(lib_name) {
		Some(lib_handle) => *lib_handle,
		None => {
			mem::drop(read_lock);
			let lib_cstr = ffi::CString::new(lib_name).unwrap();
			let lib_handle = LoadLibraryExA(
				lib_cstr.as_ptr() as *const _,
				0,
				LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
			);			
			if lib_handle == 0 {
				return Err(DylinkError::new(lib_name.to_owned(), ErrorKind::LibNotFound))
			}
			DLL_DATA
				.write()
				.unwrap()
				.insert(lib_name.to_string(), lib_handle);
			lib_handle
		}
	};
	let fn_cstr = ffi::CString::new(fn_name).unwrap();
	let maybe_fn = GetProcAddress(handle, fn_cstr.as_ptr() as *const _);
	match maybe_fn {
		Some(addr) => Ok(mem::transmute(addr)),
		None => Err(DylinkError::new(fn_name.to_owned(), ErrorKind::FnNotFound)),
	}
}
