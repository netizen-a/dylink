// Copyright (c) 2022 Jonathan "Razordor" Alan Thomason

#![allow(clippy::missing_safety_doc)]
use std::{collections::HashSet, sync};

use once_cell::sync::Lazy;

mod error;
mod ffi;
mod lazyfn;


pub use error::*;
pub use ffi::*;
pub use lazyfn::*;

// TODO: add a `#[link_name = <name>]` sub attribute to shut up clippy properly

/// Macro for generating dynamically linked functions procedurally.
/// 
/// This macro supports all ABI strings that rust natively supports.
/// For `dylink(vulkan)` mode, it is recommended to use the `"system"` ABI for cross-platform compatibility.
/// # Example
/// ```rust
/// # use dylink::dylink;
/// # type VkInstanceCreateInfo = std::ffi::c_void;
/// # type VkAllocationCallbacks = std::ffi::c_void;
/// # type VkInstance = std::ffi::c_void;
/// # type VkResult = i32;
/// #[dylink(vulkan)]
/// extern "system" {
/// 	fn vkCreateInstance(
/// 		pCreateInfo: *const VkInstanceCreateInfo,
/// 		pAllocator: *const VkAllocationCallbacks,
/// 		pInstance: *mut VkInstance,
/// 	) -> VkResult;
/// }
/// ```
pub use dylink_macro::dylink;

// I don't know how to implement wasm, so I'll just drop this here...
#[cfg(wasm)]
compile_error!("Dylink Error: Wasm is unsupported.");

// These globals are read every time a vulkan function is called for the first time,
// which occurs through `LazyFn::link`.
static VK_INSTANCE: sync::RwLock<Lazy<HashSet<ffi::VkInstance>>> =
	sync::RwLock::new(Lazy::new(|| HashSet::new()));

static VK_DEVICE: sync::RwLock<Lazy<HashSet<ffi::VkDevice>>> =
	sync::RwLock::new(Lazy::new(|| HashSet::new()));

// Used as a placeholder function pointer. This should **NEVER** be called directly,
// and promptly cast into the correct function pointer type.
pub(crate) type FnPtr = unsafe extern "system" fn() -> isize;
// The result of a dylink function
pub(crate) type Result<T> = std::result::Result<T, error::DylinkError>;

pub struct Global;
impl Global {
	// This is safe since vulkan will just discard garbage values
	/// Adds an instance to the internal HashSet.
	///
	/// Returns whether the instance was newly inserted. That is:
	///
	/// *    If the set did not previously contain this value, `true` is returned.
	/// *    If the set already contained this value, `false` is returned.
	///
	/// *note: This function returns `false` if the instance is valid and defined through dylink.*
	pub fn insert_instance(&self, instance: ffi::VkInstance) -> bool {
		//println!("insert_instance called!");
		let mut write_lock = VK_INSTANCE.write().unwrap();
		write_lock.insert(instance)
	}

	/// Removes an instance from the set. Returns whether the instance was present in the set.
	/// # Safety
	/// Using this function may break dylink's checked lifetimes!
	pub unsafe fn remove_instance(&self, instance: &ffi::VkInstance) -> bool {
		//println!("remove_instance called!");
		let mut write_lock = VK_INSTANCE.write().unwrap();
		write_lock.remove(instance)
	}

	// This is safe since vulkan will just discard garbage values
	/// Adds a device to the internal HashSet.
	///
	/// Returns whether the device was newly inserted. That is:
	///
	/// *    If the set did not previously contain this value, `true` is returned.
	/// *    If the set already contained this value, `false` is returned.
	///
	/// *note: This function returns `false` if the device is valid and defined through dylink.*
	pub fn insert_device(&self, device: ffi::VkDevice) -> bool {
		//println!("insert_device called!");
		let mut write_lock = VK_DEVICE.write().unwrap();
		write_lock.insert(device)
	}

	/// Removes a device from the set. Returns whether the value was present in the set.
	/// # Safety
	/// Using this function may break dylink's checked lifetimes!
	pub unsafe fn remove_device(&self, device: &ffi::VkDevice) -> bool {
		//println!("remove_device called!");
		let mut write_lock = VK_DEVICE.write().unwrap();
		write_lock.remove(device)
	}
}
