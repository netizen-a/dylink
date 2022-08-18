// Copyright (c) 2022 Jonathan "Razordor" Alan Thomason
#![feature(strict_provenance)]

// re-export the dylink macro
pub extern crate dylink_macro;
use std::{
	ffi, mem,
	os::raw::c_char,
	ptr,
	sync::{
		atomic::{AtomicPtr, Ordering},
		RwLock,
	},
};

pub use dylink_macro::dylink;
// perpetuate dependency for macro
#[doc(hidden)]
pub use once_cell::sync::Lazy;
use windows_sys::Win32::{
	Foundation::HINSTANCE,
	Graphics::OpenGL as gl,
	System::LibraryLoader::{GetProcAddress, LoadLibraryA},
};

#[macro_export]
macro_rules! vk_platform {
    (
        $(#[$attr:meta])+
        extern VKAPI_CALL {
            $($token:tt)+
        }
    ) => {
        #[cfg(windows)]
        vk_platform!{
            $(#[$attr])+
            extern "stdcall" {
                $($token)+
            }
        }
        #[cfg(not(windows))]
        vk_platform!{
            $(#[$attr])+
            extern "C" {
                $($token)+
            }
        }
    };
    (
        $(#[$attr:meta])+
        extern $conv:literal {
            $($token:tt)+
        }
    ) => {
        $(#[$attr])+
        extern $conv {
            $($token)+
        }
    };
    (
        $(
            $(#[$attr:meta])*
            $visibility:vis type $id:ident = extern VKAPI_CALL fn (
                $($token:tt)*
            ) $(-> $ret:ty)?;
        )+
    ) => {
		#[cfg(windows)]
		vk_platform!{
			$(
			    $(#[$attr])*
			    $visibility type $id = extern "stdcall" fn (
			        $($token)*
			    ) $(-> $ret)?;
			)+
		}
		#[cfg(not(windows))]
		vk_platform!{
			$(
			    $(#[$attr])*
			    $visibility type $id = extern "C" fn (
			        $($token)*
			    ) $(-> $ret)?;
			)+
		}
    };
	(
        $(
            $(#[$attr:meta])*
            $visibility:vis type $id:ident = extern $conv:literal fn (
                $($token:tt)*
            ) $(-> $ret:ty)?;
        )+
    ) => {
        $(
            $(#[$attr])*
            $visibility type $id = extern $conv fn (
                $($token)*
            ) $(-> $ret)?;
        )+
    };
}

type DispatchableHandle = *const ffi::c_void;
vk_platform! {
	#[allow(non_camel_case_types)]
	type PFN_vkGetProcAddr = extern VKAPI_CALL fn(DispatchableHandle, *const c_char) -> Option<fn()>;
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
pub unsafe fn vkloader(fn_name: &str) -> fn() {
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
	.expect(&format!("Dylink Error: `{fn_name}` not found!"))
}

/// `glloader` is an opengl loader specialization.
/// # Panics
/// This function might panic if the function is not found.
#[track_caller]
pub unsafe fn glloader(fn_name: &str) -> fn() {
	let c_fn_name = ffi::CString::new(fn_name).unwrap();
	let addr = gl::wglGetProcAddress(c_fn_name.as_ptr() as *const _)
		.expect(&format!("Dylink Error: `{fn_name}` not found!"));
	mem::transmute(addr)
}

/// `loader` is a generalization for all other dlls.
/// # Panics
/// This function might panic if the `lib_name` dll is not found or if the function is not found.
#[track_caller]
pub unsafe fn loader(lib_name: &str, fn_name: &str) -> fn() {
	use std::collections::HashMap;
	static DLL_DATA: Lazy<RwLock<HashMap<String, HINSTANCE>>> =
		Lazy::new(|| RwLock::new(HashMap::new()));
	let read_lock = DLL_DATA.read().unwrap();
	let handle: HINSTANCE = match read_lock.get(lib_name) {
		Some(lib_handle) => *lib_handle,
		None => {
			mem::drop(read_lock);
			let lib_cstr = ffi::CString::new(lib_name).unwrap();
			let lib_handle = LoadLibraryA(lib_cstr.as_ptr() as *const _);
			assert!(lib_handle != 0, "Dylink Error: `{lib_name}` not found!");
			DLL_DATA
				.write()
				.unwrap()
				.insert(lib_name.to_string(), lib_handle);
			lib_handle
		}
	};
	let fn_cstr = ffi::CString::new(fn_name).unwrap();
	let addr = GetProcAddress(handle, fn_cstr.as_ptr() as *const _)
		.expect("Dylink Error: `{fn_name}` not found!");
	mem::transmute(addr)
}
