// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use crate::loader::System;
use crate::{FnAddr, lazylib};
use std::ffi::CStr;
use std::sync::atomic::Ordering;
use std::{ffi, mem};
use std::sync;

// dylink_macro internally uses dylink as it's root namespace,
// but since we are in dylink the namespace is actually named `self`.
// this is just here to resolve the missing namespace issue.
extern crate self as dylink;

// I'm using a mutex here because vulkan can be sent between threads, but isn't externally synchronized (`Sync`)
pub(crate) static INSTANCES: sync::Mutex<Vec<Instance>> = sync::Mutex::new(Vec::new());
pub(crate) static DEVICES: sync::Mutex<Vec<Device>> = sync::Mutex::new(Vec::new());

#[doc(hidden)]
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instance(*mut ffi::c_void);
unsafe impl Send for Instance {}

#[doc(hidden)]
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Device(*mut ffi::c_void);
unsafe impl Send for Device {}

#[cfg(windows)]
static VK_LIB: lazylib::LazyLib<System, 1> = lazylib::LazyLib::new(unsafe{[CStr::from_bytes_with_nul_unchecked(b"vulkan-1.dll\0")]});
#[cfg(all(unix, not(target_os = "macos")))]
static VK_LIB: lazylib::LazyLib<System, 1> 	= lazylib::LazyLib::new(unsafe{[
		CStr::from_bytes_with_nul_unchecked(b"libvulkan.so.1\0"),
		CStr::from_bytes_with_nul_unchecked(b"libvulkan.so\0"),
]});
#[cfg(target_os = "macos")]
static VK_LIB: lazylib::LazyLib<System, 1> = lazylib::LazyLib::new(unsafe{[
	CStr::from_bytes_with_nul_unchecked(b"libvulkan.dylib\0"),
	CStr::from_bytes_with_nul_unchecked(b"libvulkan.1.dylib\0"),
	CStr::from_bytes_with_nul_unchecked(b"libMoltenVK.dylib\0"),
]});

// Windows and Linux are fully tested and useable as of this comment.
// MacOS should theoretically work, but it's untested.
// This function is in itself an axiom of the vulkan specialization.

#[cfg_attr(
	all(unix, not(target_os = "macos")),
	crate::dylink(any(name = "libvulkan.so.1", name = "libvulkan.so"))
)]
#[cfg_attr(
	target_os = "macos",
	crate::dylink(any(
		name = "libvulkan.dylib",
		name = "libvulkan.1.dylib",
		name = "libMoltenVK.dylib"
	))
)]
#[crate::dylink(library=VK_LIB)]
extern "system" {
	pub(crate) fn vkGetInstanceProcAddr(instance: Instance, pName: *const ffi::c_char) -> FnAddr;
}

#[allow(non_camel_case_types)]
pub(crate) type PFN_vkGetDeviceProcAddr =
	unsafe extern "system" fn(Device, *const ffi::c_char) -> FnAddr;

// vkGetDeviceProcAddr must be implemented manually to avoid recursion
#[allow(non_snake_case)]
#[inline]
pub(crate) unsafe extern "system" fn vkGetDeviceProcAddr(
	device: Device,
	name: *const ffi::c_char,
) -> FnAddr {
	/*unsafe extern "system" fn initial_fn(device: Device, name: *const ffi::c_char) -> FnAddr {
		DEVICE_PROC_ADDR.once.get_or_init(|| {
			let read_lock = INSTANCES
				.lock()
				.expect("Dylink Error: failed to get read lock");
			// check other instances if fails in case one has a higher available version number
			let raw_addr: FnAddr = read_lock
				.iter()
				.find_map(|instance| {
					vkGetInstanceProcAddr(
						*instance,
						b"vkGetDeviceProcAddr\0".as_ptr() as *const ffi::c_char,
					)
					.as_ref()
				})
				.expect("Dylink Error: failed to load `vkGetDeviceProcAddr`.")
				as FnAddr;

			*DEVICE_PROC_ADDR.addr.get() = mem::transmute(raw_addr);
			DEVICE_PROC_ADDR
				.addr_ptr
				.store(DEVICE_PROC_ADDR.addr.get(), Ordering::Relaxed);
		});
		DEVICE_PROC_ADDR(device, name)
	}

	pub(crate) static DEVICE_PROC_ADDR: lazyfn::VkLazyFn<PFN_vkGetDeviceProcAddr> =
		lazyfn::VkLazyFn::new(
			&(initial_fn as PFN_vkGetDeviceProcAddr),
			unsafe { CStr::from_bytes_with_nul_unchecked(b"vkGetDeviceProcAddr\0") },
		);
	DEVICE_PROC_ADDR(device, name)*/
	std::ptr::null()
}

pub(crate) fn vulkan_loader(fn_name: &ffi::CStr) -> FnAddr {
	let mut maybe_fn = DEVICES
		.lock()
		.expect("failed to get lock")
		.iter()
		.find_map(|device| {
			unsafe {vkGetDeviceProcAddr(*device, fn_name.as_ptr() as *const ffi::c_char).as_ref()}
		});
	maybe_fn = match maybe_fn {
		Some(addr) => return addr,
		None => INSTANCES
			.lock()
			.expect("failed to get lock")
			.iter()
			.find_map(|instance| {
				unsafe {vkGetInstanceProcAddr(*instance, fn_name.as_ptr() as *const ffi::c_char).as_ref()}
			}),
	};
	match maybe_fn {
		Some(addr) => addr,
		None => unsafe {vkGetInstanceProcAddr(
			Instance(std::ptr::null_mut()),
			fn_name.as_ptr() as *const ffi::c_char,
		)},
	}
}
