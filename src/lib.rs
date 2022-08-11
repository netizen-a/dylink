// Copyright (c) 2022 Jonathan "Razordor" Alan Thomason
#![feature(strict_provenance)]

// re-export the dylink macro
pub extern crate dylink_macro;
use std::{
	cell::UnsafeCell,
	ffi, mem,
	os::raw::c_char,
	ptr,
	sync::{
		atomic::{AtomicPtr, Ordering},
		Mutex,
	},
};

pub use dylink_macro::dylink;
// perpetuate dependency for macro
#[doc(hidden)]
pub use once_cell::sync::Lazy;
use windows_sys::{
	Win32::{
		Foundation::HINSTANCE,
		Graphics::OpenGL as gl,
		System::LibraryLoader::{GetProcAddress, LoadLibraryA},
	},
};

// The loader functions can be called on different threads by the user,
// therefore the following precautions, namely Mutex for thread safety are necessary.
type DllHandle = Mutex<UnsafeCell<Vec<(String, HINSTANCE)>>>;
static DLL_DATA: Lazy<DllHandle> = Lazy::new(|| Mutex::new(UnsafeCell::new(Vec::new())));

pub type DispatchableHandle = *const ffi::c_void;

pub struct VkContext {
	pub instance: AtomicPtr<ffi::c_void>,
	pub device:   AtomicPtr<ffi::c_void>,
}

/// `VK_CONTEXT` is loaded every time `vkloader` is called.
pub static VK_CONTEXT: VkContext = VkContext {
	instance: AtomicPtr::new(ptr::null_mut()),
	device:   AtomicPtr::new(ptr::null_mut()),
};

/// `vkloader` is a vulkan loader specialization.
#[track_caller]
pub fn vkloader(fn_name: &str) -> fn() {	
	#[allow(non_snake_case)]
	#[allow(non_upper_case_globals)]
	static vkGetInstanceProcAddr: Lazy<
		extern "stdcall" fn(DispatchableHandle, *const c_char) -> *const ffi::c_void,
	> = Lazy::new(|| unsafe { std::mem::transmute(loader("vulkan-1.dll", "vkGetInstanceProcAddr")) });
	#[allow(non_snake_case)]
	#[allow(non_upper_case_globals)]
	static vkGetDeviceProcAddr: Lazy<
		extern "stdcall" fn(DispatchableHandle, *const c_char) -> *const ffi::c_void,
	> = Lazy::new(|| unsafe {
		std::mem::transmute(vkGetInstanceProcAddr(
			VK_CONTEXT.instance.load(Ordering::Acquire),
			"vkGetDeviceProcAddr\0".as_ptr() as *const c_char,
		))
	});

	let c_fn_name = ffi::CString::new(fn_name).unwrap();
	let device = VK_CONTEXT.device.load(Ordering::Acquire);
	let instance = VK_CONTEXT.instance.load(Ordering::Acquire);
	let addr = if device.is_null() {
		vkGetInstanceProcAddr(instance, c_fn_name.as_ptr())
	} else {
		let addr = vkGetDeviceProcAddr(device, c_fn_name.as_ptr());
		if addr.is_null() {
			#[cfg(debug_assertions)]
			println!(
				"Dylink Warning: `{fn_name}` not found using `vkGetDeviceProcAddr`. Deferring \
				 call to `vkGetInstanceProcAddr`."
			);
			vkGetInstanceProcAddr(instance, c_fn_name.as_ptr())
		} else {
			addr
		}
	};	
	assert!(!addr.is_null());
	unsafe {
		mem::transmute(addr)
	}
}

/// `glloader` is an opengl loader specialization.
#[track_caller]
pub fn glloader(fn_name: &str) -> fn() {
	unsafe {
		let c_fn_name = ffi::CString::new(fn_name).unwrap();
		let addr = gl::wglGetProcAddress(c_fn_name.as_ptr() as *const _)
			.expect(&format!("Dylink Error: `{fn_name}` not found!"));			
		mem::transmute(addr)
	}
}

/// `loader` is a generalization for all other dlls.
#[track_caller]
pub fn loader(lib_name: &str, fn_name: &str) -> fn() {
	let mut lib_handle = HINSTANCE::default();
	let mut lib_found = false;
	unsafe {
		let dll_data = DLL_DATA.lock().unwrap().get();
		for lib_set in (*dll_data).iter_mut() {
			if lib_set.0 == lib_name {
				lib_found = true;
				lib_handle = lib_set.1;
			}
		}

		if !lib_found {
			let lib_cstr = ffi::CString::new(lib_name).unwrap();
			lib_handle = LoadLibraryA(lib_cstr.as_ptr() as *const _);
			assert!(lib_handle != 0, "Dylink Error: `{lib_name}` not found!");
			(*dll_data).push((lib_name.to_string(), lib_handle));
		}
	}

	let fn_cstr = ffi::CString::new(fn_name).unwrap();

	let addr: *const ffi::c_void = unsafe {
		std::mem::transmute(GetProcAddress(
			lib_handle,
			fn_cstr.as_ptr() as *const _,
		))
	};
	assert!(!addr.is_null(), "Dylink Error: `{fn_name}` not found!");
	unsafe {
		mem::transmute(addr)
	}
}
