// Copyright (c) 2022 Jonathan "Razordor" Alan Thomason
#![feature(strict_provenance)]
#![warn(fuzzy_provenance_casts)]

use std::{ffi, ptr, sync::atomic::AtomicPtr};

pub mod error;
pub mod example;
pub mod lazyfn;
pub mod loader;

pub struct VkContext {
	pub instance: AtomicPtr<ffi::c_void>,
	pub device:   AtomicPtr<ffi::c_void>,
}

// TODO: hide VK_CONTEXT behind a trait

/// `VK_CONTEXT` is loaded every time `vkloader` is called.
pub static VK_CONTEXT: VkContext = VkContext {
	instance: AtomicPtr::new(ptr::null_mut()),
	device:   AtomicPtr::new(ptr::null_mut()),
};

/// Used as a placeholder function pointer
pub type FnPtr = unsafe extern "system" fn() -> isize;
/// The result of a Dylink function
pub type Result<T> = std::result::Result<T, crate::error::DylinkError>;
