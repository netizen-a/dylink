use std::{ffi, mem, sync::RwLock};

use crate::{error::*, FnPtr, Result, VkInstance};
extern crate self as dylink;

#[dylink_macro::dylink(name = "vulkan-1")]
extern "system" {
	fn vkGetInstanceProcAddr(instance: VkInstance, pName: *const ffi::c_char) -> Option<FnPtr>;
}

/// `vkloader` is a vulkan loader specialization.
/// If `instance` is null, then `device` is ignored.
pub unsafe fn vkloader(fn_name: &'static str, instance: Option<&VkInstance>) -> Result<FnPtr> {
	let c_fn_name = ffi::CString::new(fn_name).unwrap();
	let inst = instance.map_or(VkInstance(std::ptr::null()), |r| r.clone());
	match vkGetInstanceProcAddr(inst, c_fn_name.as_ptr()) {
		Some(addr) => Ok(addr),
		None => Err(DylinkError::new(fn_name, ErrorKind::FnNotFound)),
	}
}

/// `glloader` is an opengl loader specialization.
pub unsafe fn glloader(fn_name: &'static str) -> Result<FnPtr> {
	// TODO: for unix `glXGetProcAddress`
	use windows_sys::Win32::Graphics::OpenGL::wglGetProcAddress;
	let c_fn_name = ffi::CString::new(fn_name).unwrap();
	let maybe_fn = wglGetProcAddress(c_fn_name.as_ptr() as *const _);
	match maybe_fn {
		Some(addr) => Ok(addr),
		None => Err(DylinkError::new(fn_name, ErrorKind::FnNotFound)),
	}
}

/// `loader` is a generalization for all other dlls.
pub fn loader(lib_name: &'static str, fn_name: &'static str) -> Result<FnPtr> {
	use std::collections::HashMap;

	use once_cell::sync::Lazy;
	#[cfg(windows)]
	use windows_sys::Win32::{
		Foundation::HINSTANCE,
		System::LibraryLoader::{GetProcAddress, LoadLibraryExA, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS},
	};
	
	

	static DLL_DATA: RwLock<Lazy<HashMap<String, HINSTANCE>>> =
		RwLock::new(Lazy::new(HashMap::default));

	let c_lib_name = ffi::CString::new(lib_name).unwrap();
	let c_fn_name = ffi::CString::new(fn_name).unwrap();

	let read_lock = DLL_DATA.read().unwrap();

	let handle: HINSTANCE = if let Some(handle) = read_lock.get(lib_name) {
		*handle
	} else {
		mem::drop(read_lock);

		let lib_handle = unsafe {
			#[cfg(windows)]
			{
				LoadLibraryExA(
					c_lib_name.as_ptr() as *const _,
					0,
					LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
				)
			}
			#[cfg(unix)] 
			{
				libc::dlopen(c_lib_name.as_ptr(), libc::RTLD_LOCAL | libc::RTLD_NOW) as isize
			}
		};
		if lib_handle == 0 {
			return Err(DylinkError::new(lib_name, ErrorKind::LibNotFound));
		} else {
			DLL_DATA
				.write()
				.unwrap()
				.insert(lib_name.to_owned(), lib_handle);
		}
		lib_handle
	};

	let maybe_fn = unsafe { 
		#[cfg(windows)]
		{
			GetProcAddress(handle, c_fn_name.as_ptr() as *const _) 
		}
		#[cfg(unix)]
		{
			let addr: *const libc::c_void = libc::dlsym(handle as *const libc::c_void, c_fn_name.as_ptr());
			if addr.is_null() {
				None
			} else {
				Some(std::mem::transmute(addr))
			}
		}
	};
	//let maybe_fn = libc::dlsym(handle as *const std::ffi::c_void, c_fn_name.as_ptr());
	match maybe_fn {
		Some(addr) => Ok(addr),
		None => Err(DylinkError::new(fn_name, ErrorKind::FnNotFound)),
	}
}
