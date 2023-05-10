// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use crate::lazyfn;
use crate::{FnPtr, LinkType};
use std::ffi::CStr;
use std::sync::atomic::Ordering;
use std::{ffi, mem};

// dylink_macro internally uses dylink as it's root namespace,
// but since we are in dylink the namespace is actually named `self`.
// this is just here to resolve the missing namespace issue.
extern crate self as dylink;

// FIXME: when extern types are stablized they must replace the `c_void` variation

// extern "C" {
// 	type VkInstance_T;
// 	type VkDevice_T;
// }

// #[repr(transparent)]
// #[derive(Clone, Copy, Eq, Hash, PartialEq)]
// pub struct VkInstance(pub(crate) *const VkInstance_T);

#[doc(hidden)]
#[repr(transparent)]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct VkInstance(pub(crate) *const ffi::c_void);
unsafe impl Sync for VkInstance {}
unsafe impl Send for VkInstance {}

// #[repr(transparent)]
// #[derive(Clone, Copy, Eq, Hash, PartialEq)]
// pub struct VkDevice(pub(crate) *const VkDevice_T);

#[doc(hidden)]
#[repr(transparent)]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct VkDevice(pub(crate) *const ffi::c_void);
unsafe impl Sync for VkDevice {}
unsafe impl Send for VkDevice {}

// Windows and Linux are fully tested and useable as of this comment.
// MacOS should theoretically work, but it's untested.
// This function is in itself an axiom of the vulkan specialization.
//
// Do not add `strip` here since loader::lazyfn::vulkan_loader needs it as a function pointer.
#[cfg_attr(windows, crate::dylink(name = "vulkan-1.dll"))]
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
extern "system" {
	pub(crate) fn vkGetInstanceProcAddr(
		instance: VkInstance,
		pName: *const ffi::c_char,
	) -> Option<FnPtr>;
}

#[allow(non_camel_case_types)]
pub(crate) type PFN_vkGetDeviceProcAddr =
	unsafe extern "system" fn(VkDevice, *const ffi::c_char) -> Option<FnPtr>;
#[allow(non_camel_case_types)]
pub(crate) type PFN_vkGetInstanceProcAddr =
	unsafe extern "system" fn(VkInstance, *const ffi::c_char) -> Option<FnPtr>;

// vkGetDeviceProcAddr must be implemented manually to avoid recursion
#[allow(non_snake_case)]
pub(crate) unsafe extern "system" fn vkGetDeviceProcAddr(
	device: VkDevice,
	name: *const ffi::c_char,
) -> Option<FnPtr> {
	unsafe extern "system" fn initial_fn(
		device: VkDevice,
		name: *const ffi::c_char,
	) -> Option<FnPtr> {
		//lock spinlock
		while DEVICE_PROC_ADDR.state.swap(
			DEVICE_PROC_ADDR.is_init.load(Ordering::Acquire),
			Ordering::SeqCst,
		) {
			core::hint::spin_loop()
		}

		if DEVICE_PROC_ADDR.is_init.load(Ordering::Acquire) {
			let read_lock = crate::VK_INSTANCE.read().expect("failed to get read lock");
			// check other instances if fails in case one has a higher available version number
			let fn_ptr = read_lock
				.iter()
				.find_map(|instance| {
					vkGetInstanceProcAddr(
						*instance,
						b"vkGetDeviceProcAddr\0".as_ptr() as *const ffi::c_char,
					)
				})
				.unwrap();

			let addr_ptr = DEVICE_PROC_ADDR.addr.get();
			addr_ptr.write(mem::transmute_copy(&fn_ptr));
			DEVICE_PROC_ADDR.addr_ptr.store(addr_ptr, Ordering::Relaxed);
			// unlock spinlock
			DEVICE_PROC_ADDR.is_init.store(false, Ordering::Release);
		}
		DEVICE_PROC_ADDR(device, name)
	}

	pub(crate) static DEVICE_PROC_ADDR: lazyfn::LazyFn<PFN_vkGetDeviceProcAddr> =
		lazyfn::LazyFn::new(
			&(initial_fn as PFN_vkGetDeviceProcAddr),
			unsafe { CStr::from_bytes_with_nul_unchecked(b"vkGetDeviceProcAddr\0") },
			LinkType::System(&[]),
		);
	DEVICE_PROC_ADDR(device, name)
}
