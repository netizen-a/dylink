// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
#![allow(clippy::upper_case_acronyms)]

use crate::*;
use std::ffi;
use std::marker::PhantomData;

mod selfloader;
mod system;

// self documenting hidden trait
// TODO: add `Clone` trait on next version bump
#[doc(hidden)]
pub trait FnPtr: Copy {}
impl<T: Copy> FnPtr for T {}

// LibHandle is thread-safe because it's inherently immutable, therefore don't add mutable accessors.

/// Library handle for [RTLinker]
#[derive(Debug)]
pub struct LibHandle<'a, T: ?Sized>(*mut T, PhantomData<&'a T>);
unsafe impl<T> Send for LibHandle<'_, T> where T: Send {}
unsafe impl<T> Sync for LibHandle<'_, T> where T: Sync {}

impl<'a, T> LibHandle<'a, T> {
	#[inline]
	pub fn is_invalid(&self) -> bool {
		self.0.is_null()
	}
	pub fn as_ref(&self) -> Option<&T> {
		unsafe { self.0.as_ref() }
	}
}

impl<'a, T> From<Option<&'a T>> for LibHandle<'a, T> {
	fn from(value: Option<&'a T>) -> Self {
		value
			.map(|r| Self((r as *const T).cast_mut(), PhantomData))
			.unwrap_or(Self(std::ptr::null_mut(), PhantomData))
	}
}

/// Used to specify a custom run-time linker loader for [LazyFn]
pub trait Loader<'a> {
	type Data;
	fn load_lib(lib_name: &'static ffi::CStr) -> LibHandle<'a, Self::Data>;
	fn load_sym(lib_handle: &LibHandle<'a, Self::Data>, fn_name: &'static ffi::CStr)
		-> FnAddr;
}

/// Default system linker used in [LazyFn]
#[cfg(not(wasm))]
pub struct System;

#[cfg(not(wasm))]
pub struct SelfLoader;
