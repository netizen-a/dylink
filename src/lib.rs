// Copyright (c) 2022 Jonathan "Razordor" Alan Thomason
#![feature(strict_provenance)]
#![warn(fuzzy_provenance_casts)]

// re-export the dylink macro
pub extern crate dylink_macro;
use std::{
	cell, ffi, mem,
	os::raw::c_char,
	ptr,
	sync::{
		self,
		atomic::{AtomicPtr, Ordering},
		RwLock,
	},
};

pub use dylink_macro::dylink;
use once_cell::sync::Lazy;

type DispatchableHandle = *const ffi::c_void;

use windows_sys::Win32::Foundation::PROC;

#[allow(non_camel_case_types)]
type PFN_vkGetProcAddr = extern "system" fn(DispatchableHandle, *const c_char) -> PROC;


// This is pretty much impossible to use safely without the dylink macro
#[repr(transparent)]
pub struct LazyFn<F>(cell::UnsafeCell<F>);
unsafe impl<F> Sync for LazyFn<F> {}
impl<F> LazyFn<F> {
	#[inline]
	pub const fn new(thunk: F) -> Self { Self(cell::UnsafeCell::new(thunk)) }

	/// `Once` value must be unique to each `LazyFn` instance	
	pub fn update<I>(&self, once_val: &'static sync::Once, thunk: I)
	where
		I: Fn() -> F,
	{
		once_val.call_once(|| unsafe {
			*cell::UnsafeCell::raw_get(&self.0) = thunk();
		});
	}
}
impl<F: Sized> std::ops::Deref for LazyFn<F> {
	type Target = F;

	#[inline]
	fn deref(&self) -> &Self::Target { unsafe { mem::transmute(self.0.get()) } }
}

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
/// # Panics
/// This function might panic if `vulkan-1.dll` is not found or if the function is not found.
#[track_caller]
pub unsafe fn vkloader(fn_name: &str) -> PROC {
	let device = VK_CONTEXT.device.load(Ordering::Acquire);
	let instance = VK_CONTEXT.instance.load(Ordering::Acquire);
	#[allow(non_upper_case_globals)]
	static vkGetInstanceProcAddr: Lazy<PFN_vkGetProcAddr> =
		Lazy::new(|| unsafe { mem::transmute(loader("vulkan-1.dll", "vkGetInstanceProcAddr")) });
	#[allow(non_upper_case_globals)]
	static vkGetDeviceProcAddr: Lazy<PFN_vkGetProcAddr> = Lazy::new(|| unsafe {
		mem::transmute(vkGetInstanceProcAddr(
			VK_CONTEXT.instance.load(Ordering::Acquire),
			"vkGetDeviceProcAddr\0".as_ptr() as *const c_char,
		))
	});

	let c_fn_name = ffi::CString::new(fn_name).unwrap();
	if device.is_null() {
		vkGetInstanceProcAddr(instance, c_fn_name.as_ptr())
	} else {
		vkGetDeviceProcAddr(device, c_fn_name.as_ptr())
			.or_else(|| vkGetInstanceProcAddr(instance, c_fn_name.as_ptr()))
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
	use windows_sys::Win32::{
		System::LibraryLoader::{GetProcAddress, LoadLibraryExA, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS},
		Foundation::HINSTANCE,
	};	
	
	use std::collections::HashMap;

	static DLL_DATA: Lazy<RwLock<HashMap<String, HINSTANCE>>> =
		Lazy::new(|| RwLock::new(HashMap::new()));

	let read_lock = DLL_DATA.read().unwrap();
	let handle: HINSTANCE = match read_lock.get(lib_name) {
		Some(lib_handle) => *lib_handle,
		None => {
			mem::drop(read_lock);
			let lib_cstr = ffi::CString::new(lib_name).unwrap();
			let lib_handle = LoadLibraryExA(lib_cstr.as_ptr() as *const _, 0, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS);
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
