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

pub mod cell;
pub mod os;
pub mod sync;

use std::{ffi, io, mem, path, sync::atomic::AtomicPtr};

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

#[cfg(unix)]
use os::unix::{dylib_close, dylib_close_and_exit, dylib_open, dylib_symbol, dylib_this};
#[cfg(windows)]
use os::windows::{dylib_close, dylib_close_and_exit, dylib_open, dylib_symbol, dylib_this};

#[doc = include_str!("../README.md")]
#[cfg(all(doctest, windows))]
struct ReadmeDoctests;

#[repr(C)]
pub struct Sym {
	_data: [u8; 0],
	_marker: core::marker::PhantomData<(*mut u8, std::marker::PhantomPinned)>,
}

#[inline]
const fn handle_to_lib(handle: *mut ffi::c_void) -> Library {
	Library(AtomicPtr::new(handle))
}

// primitive type for handling library handles
// Library should be treated as Arc
#[derive(Debug)]
pub struct Library(AtomicPtr<ffi::c_void>);

impl Library {
	// default way to open library
	pub fn open<P: AsRef<path::Path>>(path: P) -> io::Result<Self> {
		unsafe { dylib_open(path.as_ref()) }.map(handle_to_lib)
	}

	pub fn this() -> io::Result<Self> {
		unsafe { dylib_this() }.map(handle_to_lib)
	}

	pub fn symbol<'a>(&'a mut self, name: &'a str) -> io::Result<&'a Sym> {
		unsafe { dylib_symbol(*self.0.get_mut(), name) }
	}
}

impl Drop for Library {
	fn drop(&mut self) {
		unsafe {
			let _ = dylib_close(*self.0.get_mut());
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

// This is the preferred way to close libraries and exit on windows, but it also works for unix.
pub fn close_and_exit(lib: Library, exit_code: u32) -> ! {
	let mut lib = mem::ManuallyDrop::new(lib);
	let handle = lib.0.get_mut();
	unsafe { dylib_close_and_exit(*handle, exit_code) }
}
