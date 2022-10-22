use std::{
	ffi, mem,
	sync::{atomic::Ordering, RwLock},
};

use once_cell::sync::Lazy;
use windows_sys::Win32::Foundation::PROC;

use crate::{example::*, lazyfn::*, VK_CONTEXT};

//TODO: change PROC to Result<extern "system" fn(), DylinkError>

/// `vkloader` is a vulkan loader specialization.
/// # Panics
/// This function might panic if `vulkan-1.dll` is not found or if the function is not found.
#[track_caller]
pub unsafe fn vkloader(fn_name: &str) -> PROC {
	let device = VK_CONTEXT.device.load(Ordering::Acquire);
	let instance = VK_CONTEXT.instance.load(Ordering::Acquire);
	let c_fn_name = ffi::CString::new(fn_name).unwrap();
	if let Some(device) = std::ptr::NonNull::new(device) {
		vkGetDeviceProcAddr(device, c_fn_name.as_ptr())
			.or_else(|| vkGetInstanceProcAddr(instance, c_fn_name.as_ptr()))
	} else {
		vkGetInstanceProcAddr(instance, c_fn_name.as_ptr())
	}
}

/// `glloader` is an opengl loader specialization.
/// # Panics
/// This function might panic if the function is not found.
#[track_caller]
pub unsafe fn glloader(fn_name: &str) -> PROC {
	use windows_sys::Win32::Graphics::OpenGL::wglGetProcAddress;
	let c_fn_name = ffi::CString::new(fn_name).unwrap();
	let addr = wglGetProcAddress(c_fn_name.as_ptr() as *const _);
	mem::transmute(addr)
}

/// `loader` is a generalization for all other dlls.
/// # Panics
/// This function might panic if the `lib_name` dll is not found or if the function is not found.
#[track_caller]
pub unsafe fn loader(lib_name: &str, fn_name: &str) -> PROC {
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
			assert!(lib_handle != 0, "Dylink Error: `{lib_name}` not found!");
			DLL_DATA
				.write()
				.unwrap()
				.insert(lib_name.to_string(), lib_handle);
			lib_handle
		}
	};
	let fn_cstr = ffi::CString::new(fn_name).unwrap();
	GetProcAddress(handle, fn_cstr.as_ptr() as *const _)
}
