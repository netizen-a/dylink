// Copyright (c) 2022 Jonathan "Razordor" Alan Thomason
#![allow(clippy::missing_safety_doc)]
use core::{ptr, sync::atomic::AtomicPtr};

pub mod error;
pub mod example;
pub mod lazyfn;

// TODO: make this work through more than just windows
#[cfg(windows)]
pub mod loader;

pub struct VkContext {
	pub instance: AtomicPtr<()>,
	pub device:   AtomicPtr<()>,
}

/// This global is read every time a vulkan function is called for the first time,
/// which silently occurs through `LazyFn::link_lib`.
pub static VK_CONTEXT: VkContext = VkContext {
	instance: AtomicPtr::new(ptr::null_mut()),
	device:   AtomicPtr::new(ptr::null_mut()),
};

/// Used as a placeholder function pointer. This should **NEVER** be called directly,
/// and promptly cast into the correct function pointer type.
pub type FnPtr = unsafe extern "system" fn() -> isize;
/// The result of a Dylink function
pub type Result<T> = core::result::Result<T, crate::error::DylinkError>;
