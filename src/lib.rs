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
//! The appropriate ABI to use with the vulkan loader is `extern "system"`. Unlike generalized loading, the vulkan specialization loads
//! functions indirectly through `vkGetInstanceProcAddr`, and when applicable `vkGetDeviceProcAddr`, to retrieve vulkan addresses properly.
//! The internal copies of the instance and device handles are only stored until destroyed through a dylink generated vulkan function.
//!
//! *Note: Due to how dylink handles loading, `vkCreateInstance`, `vkDestroyInstance`, `vkCreateDevice`, and `vkDestroyDevice` are
//! incompatible with the `strip=true` macro argument.*
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
//! ```rust
//! # use dylink::dylink;
//! #[dylink(any(name = "example_lib.so", name = "example_lib.so.1"))]
//! extern "C" {
//!     fn my_function();
//! }
//! ```
//!
//! # Checking for Libraries
//! The greatest strength in dynamically linking at run-time is the ability to recover when libraries are missing.
//! This can even include when all libraries in the configuration predicate mentioned above fails. To handle this
//! problem dylink provides a `strip` argument that you can use with the macro to strip the abstraction and
//! leverage the underlying static variable's member functions.
//!
//! *Note: Stripping the abstraction does not necessarily make it cheaper, because dylink is designed to inline the abstraction for you.*
//! ```rust
//! # use dylink::dylink;
//! #[dylink(name = "example.so", strip=true)]
//! extern "C" {
//!     fn my_function();
//! }
//!
//! fn main() {
//!     match my_function.try_link() {
//!         Ok(function) => unsafe {function()},
//!         Err(reason) => println!("{reason}"),
//!     }
//! }
//! ```
//!
//! The `strip` argument as mentioned above has an unfortunate caveat of not being documentation friendly and cannot be freely
//! passed around as a function pointer since the function will use the `LazyFn` wrapper, which is the fundemental type of the
//! dylink crate. Although stripped abstractions cannot be passed around like `fn` pointers they can still be called like one.
//! However, without explicitly checking if the library exists at any point, it may still panic with an appropriate error
//! message if the library is really missing.
//! ```should_panic
//! # use dylink::dylink;
//! #[dylink(name = "missing_library.dll", strip=true)]
//! extern "C" {
//!     fn my_function();
//! }
//!
//! fn main() {
//!     unsafe { my_function() } // panics since the library is missing
//! }
//! ```
//!
//! # About Library Unloading
//! Shared library unloading is extremely cursed, always unsafe, and we don't even try to support it.
//! Unloading a library means not only are all loaded dylink functions invalidated, but functions loaded from **ALL**
//! crates in the project are also invalidated, which will immediately lead to segfaults... a lot of them.

use std::sync;

use std::ffi;

mod error;
mod lazyfn;
mod vulkan;

pub use error::*;
pub use lazyfn::*;
pub use vulkan::{VkDevice, VkInstance};

/// Macro for generating dynamically linked functions procedurally.
///
/// Refer to crate level documentation for more information.
pub use dylink_macro::dylink;

#[doc = include_str!("../README.md")]
#[cfg(all(doctest, windows))]
pub struct ReadmeDoctests;

// I don't know how to implement wasm, so I'll just drop this here...
#[cfg(wasm)]
compile_error!("Dylink Error: Wasm is unsupported.");

// These globals are read every time a vulkan function is called for the first time,
// which occurs through `LazyFn::link`.
static VK_INSTANCE: sync::RwLock<Vec<vulkan::VkInstance>> = sync::RwLock::new(Vec::new());

static VK_DEVICE: sync::RwLock<Vec<vulkan::VkDevice>> = sync::RwLock::new(Vec::new());

// Used as a placeholder function pointer. This should **NEVER** be called directly,
// and promptly cast into the correct function pointer type.
pub(crate) type FnPtr = unsafe extern "system" fn() -> isize;

// The result of a dylink function
pub(crate) type DylinkResult<T> = Result<T, error::DylinkError>;

// TODO: Make the `Global` struct below public when name is picked out

/// The global context for specializations.
///
/// This implicitly controls how the specialization `#[dylink(vulkan)]` handles function loading.
/// This Global is injected when building specializations, but is excluded when building generalizations,
/// such as `#[dylink(name = "my_lib.so")]`.
#[doc(hidden)]
pub struct Global;
impl Global {
	// This is safe since vulkan will just discard garbage values
	/// Adds an instance to the internal Vec.
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
		match write_lock.binary_search(&instance) {
			Ok(_) => false,
			Err(index) => {
				write_lock.insert(index, instance);
				true
			}
		}
	}

	/// Removes an instance from the set. Returns whether the instance was present in the set.
	/// # Safety
	/// Using this function may break dylink's checked lifetimes!
	pub unsafe fn remove_instance(&self, instance: &vulkan::VkInstance) -> bool {
		//println!("remove_instance called!");
		let mut write_lock = VK_INSTANCE.write().unwrap();
		match write_lock.binary_search(instance) {
			Ok(index) => {
				write_lock.remove(index);
				true
			}
			Err(_) => false,
		}
	}

	// This is safe since vulkan will just discard garbage values
	/// Adds a device to the internal Vec.
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
		match write_lock.binary_search(&device) {
			Ok(_) => false,
			Err(index) => {
				write_lock.insert(index, device);
				true
			}
		}
	}

	/// Removes a device from the set. Returns whether the value was present in the set.
	/// # Safety
	/// Using this function may break dylink's checked lifetimes!
	pub unsafe fn remove_device(&self, device: &vulkan::VkDevice) -> bool {
		//println!("remove_device called!");
		let mut write_lock = VK_DEVICE.write().unwrap();
		match write_lock.binary_search(device) {
			Ok(index) => {
				write_lock.remove(index);
				true
			}
			Err(_) => false,
		}
	}
}

/// Opaque pointer sized library handle
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct LibHandle<T>(*const T);
unsafe impl <T> Send for LibHandle<T> {}
unsafe impl <T> Sync for LibHandle<T> {}

impl<T> LibHandle<T> {
	#[inline]
	pub fn is_invalid(&self) -> bool {
		self.0.is_null()
	}
	pub(crate) fn as_opaque(&self) -> LibHandle<ffi::c_void> {
		LibHandle(self.0.cast())
	}
	pub fn as_ref(&self) -> Option<&T> {
		unsafe { self.0.as_ref() }
	}
}

impl<T> From<Option<&T>> for LibHandle<T> {
	fn from(value: Option<&T>) -> Self {
		value
			.map(|r| Self((r as *const T).cast()))
			.unwrap_or(Self(std::ptr::null()))
	}
}

/// Used to specify a custom linker loader for `LazyFn`
pub trait RTLinker {
	type Data;
	fn load_lib(lib_name: &ffi::CStr) -> LibHandle<Self::Data>;
	fn load_sym(lib_handle: &LibHandle<Self::Data>, fn_name: &ffi::CStr) -> Option<FnPtr>;
}
