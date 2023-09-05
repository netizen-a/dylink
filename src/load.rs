// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use std::sync::atomic::AtomicPtr;

use std::io;
use std::path::Path;

mod this;
mod sys;

/// Used to specify the run-time linker loader constraint for [`sync::Library`](crate::sync::Library) and [`cell::Library`](crate::cell::Library).
/// `Loader` can also be used to make custom loaders.
pub unsafe trait Loader: Send + Sized {
	/// Attempts to open a shared library.
	///
	/// Returns `Ok` if success, otherwise `Err`.
	unsafe fn open<P: AsRef<Path>>(path: P) -> io::Result<Self>;
	/// Retrieves raw symbol from shared library.
	///
	/// If successful, returns a valid address to symbol, otherwise
	/// returns a `null` pointer.
	unsafe fn sym(&self, symbol: &str) -> *const ();
}

/// An object providing access to an open shared library on the filesystem.
///
/// This is a basic library loader primitive designed to be used with [`sync::Library`](crate::sync::Library) and [`cell::Library`](crate::cell::Library).
#[derive(Debug)]
pub struct System(AtomicPtr<std::ffi::c_void>);

/// An object providing access to libraries currently loaded by this process.
///
/// # Unix Platform
///
/// The unix implementation uses `RTLD_DEFAULT`, which does not require additional input.
/// Since `Library` and `CloseableLibrary` still require at least one library name, so a dummy
/// value must be used.
///
/// # Windows Platform
///
/// The windows implementation must specify, which libraries the `This` structure shall attempt
/// to load from.
#[derive(Debug)]
pub struct This(AtomicPtr<std::ffi::c_void>);
