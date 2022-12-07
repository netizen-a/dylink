// Copyright (c) 2022 Jonathan "Razordor" Alan Thomason
#![cfg_attr(feature = "opaque_types", feature(extern_types))]
#![allow(clippy::missing_safety_doc)]
use std::{
	sync,
	collections::HashSet
};

use once_cell::sync::Lazy;

pub mod error;
pub mod lazyfn;

// TODO: make this work through more than just windows
#[cfg(windows)]
pub mod loader;

/// This global is read every time a vulkan function is called for the first time,
/// which silently occurs through `LazyFn::link_lib`.
static VK_INSTANCE: sync::RwLock<Lazy<HashSet<VkInstance>>> = sync::RwLock::new(Lazy::new(|| HashSet::new()));

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
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct VkInstance(pub(crate) *const VkInstance_T);

#[cfg(not(feature = "opaque_types"))]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct VkInstance(pub(crate) *const std::ffi::c_void);

// pretend VkInstance is not a pointer. dylink never dereferences the contents (because it can't), 
// so there shouldn't be aliasing problems.
unsafe impl Sync for VkInstance {}
unsafe impl Send for VkInstance {}

pub struct Global;
impl Global {
	pub fn insert_instance(&self, instance: VkInstance)
	{
		let mut write_lock = VK_INSTANCE.write().unwrap();
		write_lock.insert(instance);
	}
	pub fn remove_instance(&self, instance: VkInstance)	
	{
		let mut write_lock = VK_INSTANCE.write().unwrap();
		write_lock.remove(&instance);
	}
}