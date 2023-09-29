// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! Dylink provides a run-time dynamic linking framework for lazily evaluating shared libraries.
//! When functions are loaded they are evaluated through a thunk for first time calls, which loads the function
//! from its respective library. Preceeding calls after initialization have no overhead or additional branching
//! checks, since the thunk is replaced by the loaded function.
//!
//! # Basic Example
//!
//! ```rust
//! use dylink::*;
//!
//! static KERNEL32: sync::LibLock = sync::LibLock::new(&["Kernel32.dll"]);
//!
//! #[dylink(library=KERNEL32)]
//! extern "system" {
//!     fn GetLastError() -> u32;
//!     fn SetLastError(_: u32);
//! }
//! ```

pub mod sync;
pub mod cell;
mod os;

use std::{path, io, ffi, sync::atomic::AtomicPtr};

/// Macro for generating shared symbol thunks procedurally.
///
/// May currently be used in 2 patterns:
/// * foreign modules
/// * foreign functions
///
/// More may patterns may be added in the future if needed.
/// # Examples
///```rust
/// use dylink::*;
/// static FOOBAR: sync::LibLock = sync::LibLock::new(&["foobar.dll"]);
///
/// // foreign module pattern
/// #[dylink(library=FOOBAR)]
/// extern "system" {
///     fn foo();
/// }
///
/// // foreign function pattern
/// #[dylink(library=FOOBAR)]
/// extern "system" fn bar();
///```
pub use dylink_macro::dylink;

#[cfg(windows)]
use os::windows::{dylib_open, dylib_close, dylib_symbol, dylib_this};
#[cfg(unix)]
use os::unix::{dylib_open, dylib_close, dylib_symbol, dylib_this};

#[doc = include_str!("../README.md")]
#[cfg(all(doctest, windows))]
struct ReadmeDoctests;


#[repr(C)]
pub struct Sym {
	_data: [u8; 0],
	_marker: core::marker::PhantomData<(*mut u8, std::marker::PhantomPinned)>,
}

/// An object providing access to an open dynamic library.
#[derive(Debug)]
pub struct Library(AtomicPtr<ffi::c_void>);

impl Library {
    pub fn open<P: AsRef<path::Path>>(path: P) -> io::Result<Self> {
		unsafe {dylib_open(path.as_ref())}
			.and_then(|handle| Ok(Self(AtomicPtr::new(handle))))
    }

	pub fn this() -> io::Result<Self> {
		unsafe {dylib_this()}
			.and_then(|handle| Ok(Self(AtomicPtr::new(handle))))
	}

	pub fn symbol<'a>(&'a mut self, name: &'a str) -> io::Result<&'a Sym> {
		unsafe {dylib_symbol(*self.0.get_mut(), name)}
	}
}

impl Drop for Library {
	fn drop(&mut self) {
		unsafe {
    		let _ = dylib_close(*self.0.get_mut()).unwrap();
		}
	}
}

#[macro_export]
macro_rules! lib {
	($name:literal $(, alt_names:literal)*) => {
		$crate::Library::open($name)
		$(.or_else(||$crate::Library::open($name)))*
	};
}