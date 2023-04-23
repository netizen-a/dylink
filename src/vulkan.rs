// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
use std::{ffi, mem};
use crate::FnPtr;
use crate::lazyfn;

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

// vkGetDeviceProcAddr must be implemented manually to avoid recursion
#[allow(non_upper_case_globals)]
pub(crate) static vkGetDeviceProcAddr: lazyfn::LazyFn<
	unsafe extern "system" fn(VkDevice, *const ffi::c_char) -> Option<FnPtr>,
> = lazyfn::LazyFn::new({
	unsafe extern "system" fn initial_fn(
		device: VkDevice,
		name: *const ffi::c_char,
	) -> Option<FnPtr> {
		vkGetDeviceProcAddr.once.call_once(|| {
			let read_lock = crate::VK_INSTANCE.read().expect("failed to get read lock");			
			// check other instances if fails in case one has a higher available version number
			let fn_ptr = read_lock
				.iter()
				.find_map(|instance| {
					vkGetInstanceProcAddr(
						*instance, 
						b"vkGetDeviceProcAddr\0".as_ptr() as *const ffi::c_char
					)
				});
			*std::cell::UnsafeCell::raw_get(&vkGetDeviceProcAddr.addr) = mem::transmute(fn_ptr);
		});
		vkGetDeviceProcAddr(device, name)
	}initial_fn}
);