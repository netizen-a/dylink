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
use windows::{
	core,
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

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct DispatchableHandle(pub *const ffi::c_void);

/// Context is used in place of `VkInstance` to invoke the vulkan specialization.
struct Context {
	instance: AtomicPtr<ffi::c_void>,
	device:   AtomicPtr<ffi::c_void>,
}

/// Setting instance allows dylink to load Vulkan functions.
#[inline]
pub fn set_instance(inst: DispatchableHandle) {
	CONTEXT.instance.store(inst.0 as *mut _, Ordering::Release);
}

/// Setting device to a non-null value lets Dylink call `vkGetDeviceProcAddr`.    
#[inline]
pub fn set_device(dev: DispatchableHandle) {
	CONTEXT.device.store(dev.0 as *mut _, Ordering::Release);
}

#[inline]
pub fn get_instance() -> DispatchableHandle {
	DispatchableHandle(CONTEXT.instance.load(Ordering::Acquire))
}

#[inline]
pub fn get_device() -> DispatchableHandle {
	DispatchableHandle(CONTEXT.device.load(Ordering::Acquire))
}

static CONTEXT: Context = Context {
	instance: AtomicPtr::new(ptr::null_mut()),
	device:   AtomicPtr::new(ptr::null_mut()),
};

/// `vkloader` is a vulkan loader specialization.
pub fn vkloader(fn_name: &str) -> Option<fn()> {
	// let context = context.borrow();
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
			get_instance(),
			"vkGetDeviceProcAddr\0".as_ptr() as *const c_char,
		))
	});

	let c_fn_name = ffi::CString::new(fn_name).unwrap();
	let device = get_device();
	let addr = if device.0.is_null() {
		vkGetInstanceProcAddr(get_instance(), c_fn_name.as_ptr())
	} else {
		let addr = vkGetDeviceProcAddr(device, c_fn_name.as_ptr());
		if addr.is_null() {
			#[cfg(debug_assertions)]
			println!(
				"Dylink Warning: `{fn_name}` not found using `vkGetDeviceProcAddr`. Deferring \
				 call to `vkGetInstanceProcAddr`."
			);
			vkGetInstanceProcAddr(get_instance(), c_fn_name.as_ptr())
		} else {
			addr
		}
	};
	if addr.is_null() {
		None
	} else {
		unsafe { Some(mem::transmute(addr)) }
	}
}

/// `glloader` is an opengl loader specialization.
pub fn glloader(fn_name: &str) -> *const ffi::c_void {
	unsafe {
		let c_fn_name = ffi::CString::new(fn_name).unwrap();
		let addr = gl::wglGetProcAddress(core::PCSTR(c_fn_name.as_ptr() as *const _))
			.expect(&format!("Dylink Error: `{fn_name}` not found!"));
		mem::transmute(addr)
	}
}

/// `loader` is a generalization for all other dlls.
pub fn loader(lib_name: &str, fn_name: &str) -> ptr::NonNull<ffi::c_void> {
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
			lib_handle = LoadLibraryA(core::PCSTR(lib_cstr.as_ptr() as *const _))
				.expect("Dylink Error: `{}` not found!");
			(*dll_data).push((lib_name.to_string(), lib_handle));
		}
	}

	let fn_cstr = ffi::CString::new(fn_name).unwrap();

	let addr: *const ffi::c_void = unsafe {
		std::mem::transmute(GetProcAddress(
			lib_handle,
			core::PCSTR(fn_cstr.as_ptr() as *const _),
		))
	};
	ptr::NonNull::new(addr as *mut _).expect(&format!("Dylink Error: `{fn_name}` not found!"))
}
