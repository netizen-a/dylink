// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use crate::*;
use core::ffi;

#[cfg(feature = "std")]
mod self_loader;
#[cfg(feature = "std")]
mod sys_loader;

// self documenting hidden trait
// TODO: add `Clone` trait on next version bump
#[doc(hidden)]
pub trait FnPtr: Copy + Clone {}
impl<T: Copy + Clone> FnPtr for T {}

pub trait LibHandle: Send {
	fn is_invalid(&self) -> bool;
}

/// Used to specify the run-time linker loader constraint for [LazyLib]
///
/// This trait must never panic, or a potential deadlock may occur when used with [LazyLib].
pub trait Loader
where
	Self::Handle: LibHandle,
{
	type Handle;
	fn load_lib(lib_name: &'static ffi::CStr) -> Self::Handle;
	fn load_sym(lib_handle: &Self::Handle, fn_name: &'static ffi::CStr) -> FnAddr;
}

/// Default system loader used in [LazyLib]
#[cfg(feature = "std")]
pub struct SysLoader;

#[cfg(feature = "std")]
pub struct SelfLoader;