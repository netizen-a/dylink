// Copyright (c) 2022 Jonathan "Razordor" Alan Thomason
#![feature(strict_provenance)]
#![feature(core_intrinsics)]
#![warn(fuzzy_provenance_casts)]

use std::{ffi, ptr, sync::atomic::AtomicPtr};

pub mod error;
pub mod example;
pub mod lazyfn;
pub mod loader;

// Re-export
pub use dylink_macro::dylink;

pub struct VkContext {
	pub instance: AtomicPtr<ffi::c_void>,
	pub device:   AtomicPtr<ffi::c_void>,
}

/// `VK_CONTEXT` is loaded every time `vkloader` is called.
pub static VK_CONTEXT: VkContext = VkContext {
	instance: AtomicPtr::new(ptr::null_mut()),
	device:   AtomicPtr::new(ptr::null_mut()),
};

// Used as a placeholder function pointer
pub type FnPtr = Option<unsafe extern "system" fn() -> isize>;