// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
#![allow(clippy::upper_case_acronyms)]

use crate::*;
use std::ffi;

mod selfloader;
mod system;

// self documenting hidden trait
// TODO: add `Clone` trait on next version bump
#[doc(hidden)]
pub trait FnPtr: Copy + Clone {}
impl<T: Copy + Clone> FnPtr for T {}

pub trait LibHandle: Send {
	fn is_invalid(&self) -> bool;
}

/// Used to specify a custom run-time linker loader for [LazyFn]
pub trait Loader<'a>
where
	Self::Handle: LibHandle + 'a,
{
	type Handle;
	fn load_lib(lib_name: &'static ffi::CStr) -> Self::Handle;
	fn load_sym(lib_handle: &Self::Handle, fn_name: &'static ffi::CStr) -> FnAddr;
}

/// Default system linker used in [LazyFn]
#[cfg(not(wasm))]
pub struct System;

#[cfg(not(wasm))]
pub struct SelfLoader;
