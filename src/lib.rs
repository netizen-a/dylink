// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

//! This macro is designed to make loading functions from `.dll`s, and `.so` files easy and convenient.
//! `dylink` can make use of the configuration predicate `any`, which can be used to fallback to alternative shared libraries as needed.
//!
//! # General Example
//! ```rust
//! use dylink::dylink;
//!
//! #[dylink(name = "Kernel32.dll")]
//! extern "system" {
//!     fn GetLastError() -> u32;
//! }
//! ```
//!
//! # Vulkan Specialization
//! There is also a builtin vulkan loader option in the macro as well, which automatically looks for the shared library where expected.
//! The appropriate ABI to use with the vulkan loader is `extern "system"`.
//! ```rust
//! use dylink::dylink;
//!
//! # type VkInstanceCreateInfo = std::ffi::c_void;
//! # type VkAllocationCallbacks = std::ffi::c_void;
//! # type VkInstance = std::ffi::c_void;
//! # type VkResult = i32;
//! #[dylink(vulkan)]
//! extern "system" {
//!     fn vkCreateInstance(
//!         pCreateInfo: *const VkInstanceCreateInfo,
//!         pAllocator: *const VkAllocationCallbacks,
//!         pInstance: *mut VkInstance,
//!     ) -> VkResult;
//! }
//! ```
//!
//! # Configuration Predicates
//! Dylink can also accept predicated disjunctions in an idiomatic manner by making use of the `any` function.
//! `any()` uses short-circuit logic to check for the existance of shared libraries.
//!
//! Note: `any()` only handles the library predicate, and not the function predicate.
//! This means that if the library is found, but the function is not, a panic will occur.
//! ```rust
//! #[dylink(any(name = "example_lib.so", name = "example_lib.so.1"))]
//! extern "C" {
//!     fn my_function();
//! }
//! ```

use std::{collections::HashSet, sync};

use once_cell::sync::Lazy;

mod error;
mod lazyfn;
mod vulkan;

pub use error::*;
pub use lazyfn::*;
pub use vulkan::{VkDevice, VkInstance};

/// Macro for generating dynamically linked functions procedurally.
///
/// This macro supports all ABI strings that rust natively supports.
/// # Example
/// ```rust
/// # use dylink::dylink;
/// # type VkInstanceCreateInfo = std::ffi::c_void;
/// # type VkAllocationCallbacks = std::ffi::c_void;
/// # type VkInstance = std::ffi::c_void;
/// # type VkResult = i32;
/// #[dylink(vulkan)]
/// extern "system" {
///     fn vkCreateInstance(
///         pCreateInfo: *const VkInstanceCreateInfo,
///         pAllocator: *const VkAllocationCallbacks,
///         pInstance: *mut VkInstance,
///     ) -> VkResult;
/// }
/// ```
pub use dylink_macro::dylink;

// I don't know how to implement wasm, so I'll just drop this here...
#[cfg(wasm)]
compile_error!("Dylink Error: Wasm is unsupported.");

// These globals are read every time a vulkan function is called for the first time,
// which occurs through `LazyFn::link`.
static VK_INSTANCE: sync::RwLock<Lazy<HashSet<vulkan::VkInstance>>> =
	sync::RwLock::new(Lazy::new(HashSet::new));

static VK_DEVICE: sync::RwLock<Lazy<HashSet<vulkan::VkDevice>>> =
	sync::RwLock::new(Lazy::new(HashSet::new));

// Used as a placeholder function pointer. This should **NEVER** be called directly,
// and promptly cast into the correct function pointer type.
pub(crate) type FnPtr = unsafe extern "system" fn() -> isize;
// The result of a dylink function
pub(crate) type Result<T> = std::result::Result<T, error::DylinkError>;

// TODO: Make the `Global` struct below public when extern types are stable.
//		 The name `Global` is still TBD.

/// The global context for specializations.
///
/// This implicitly controls how the specialization `#[dylink(vulkan)]` handles function loading.
/// This Global is injected when building specializations, but is excluded when building generalizations,
/// such as `#[dylink(name = "my_lib.so")]`.
#[doc(hidden)]
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
	pub fn insert_instance(&self, instance: vulkan::VkInstance) -> bool {
		//println!("insert_instance called!");
		let mut write_lock = VK_INSTANCE.write().unwrap();
		write_lock.insert(instance)
	}

	/// Removes an instance from the set. Returns whether the instance was present in the set.
	/// # Safety
	/// Using this function may break dylink's checked lifetimes!
	pub unsafe fn remove_instance(&self, instance: &vulkan::VkInstance) -> bool {
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
	pub fn insert_device(&self, device: vulkan::VkDevice) -> bool {
		//println!("insert_device called!");
		let mut write_lock = VK_DEVICE.write().unwrap();
		write_lock.insert(device)
	}

	/// Removes a device from the set. Returns whether the value was present in the set.
	/// # Safety
	/// Using this function may break dylink's checked lifetimes!
	pub unsafe fn remove_device(&self, device: &vulkan::VkDevice) -> bool {
		//println!("remove_device called!");
		let mut write_lock = VK_DEVICE.write().unwrap();
		write_lock.remove(device)
	}
}
