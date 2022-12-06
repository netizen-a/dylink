// Copyright (c) 2022 Jonathan "Razordor" Alan Thomason
#![cfg_attr(feature = "opaque_types", feature(extern_types))]
#![allow(clippy::missing_safety_doc)]
use std::{
	ffi, ptr,
	sync::atomic::{AtomicPtr, Ordering},
};

pub mod error;
pub mod lazyfn;

// TODO: make this work through more than just windows
#[cfg(windows)]
pub mod loader;

/// This global is read every time a vulkan function is called for the first time,
/// which silently occurs through `LazyFn::link_lib`.
static VK_INSTANCE: AtomicPtr<ffi::c_void> = AtomicPtr::new(ptr::null_mut());

/// Used as a placeholder function pointer. This should **NEVER** be called directly,
/// and promptly cast into the correct function pointer type.
pub type FnPtr = unsafe extern "system" fn() -> isize;
/// The result of a dylink function
pub type Result<T> = std::result::Result<T, error::DylinkError>;

#[cfg(feature = "opaque_types")]
extern "C" {
	#[doc(hidden)]
	pub type VkInstance_T;
}
#[cfg(feature = "opaque_types")]
pub type VkInstance = *const VkInstance_T;

#[cfg(not(feature = "opaque_types"))]
pub type VkInstance = *const ffi::c_void;

/// # Safety
/// This function directly impacts vulkan functions being loaded, by editing an internal
/// static variable that is used to call `vkGetInstanceProcAddr`.
/// The lifetime of VkInstance is determined when you initialize vkCreateInstance and vkDestroyInstance.
/// When `vkDestroyInstance` is called, no more functions may be initialized. You may use this function
/// in case you have a corner case such that you need to create additional functions using a different instance.
pub unsafe fn use_instance(instance: VkInstance) {
	VK_INSTANCE.store(instance.cast::<ffi::c_void>().cast_mut(), Ordering::Release);
}
