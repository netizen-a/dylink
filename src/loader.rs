// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use crate::*;
use std::io;

#[cfg(any(windows, unix, doc))]
mod self_loader;
#[cfg(any(windows, unix, doc))]
mod sys_loader;

/// Used to specify the run-time linker loader constraint for [`Library`].
/// `Loader` can also be used to make custom loaders.
pub unsafe trait Loader: Send + Sized {
	/// Opens a shared library at `path`.
	///
	/// Returns `Some` if success, otherwise `None`.
	unsafe fn open(path: &str) -> Option<Self>;
	/// Retrieves symbol from shared library.
	///
	/// If success [`SymAddr`] resolves to a valid pointer, otherwise is `null`.
	unsafe fn find_symbol(&self, symbol: &str) -> SymAddr;
}

/// This trait is similar to the `Drop` trait, which frees resources.
/// Unlike the `Drop` trait, `Close` must assume there side affects when closing a library.
/// As a consequence of these side affects `close` is marked as `unsafe`.
///
/// *Note: Closing a library is always considered super unsafe.*
pub unsafe trait Close: Loader {
	unsafe fn close(self) -> io::Result<()>;
}

/// A system library loader.
///
/// This is a basic library loader primitive designed to be used with [`Library`].
#[cfg(any(windows, unix, doc))]
pub struct SystemLoader(*mut core::ffi::c_void);

/// A system self loader.
///
/// This loader is retrieves symbols from libraries currently loaded by this process.
///
/// # Unix Platform
///
/// The unix implementation uses `RTLD_DEFAULT`, which does not require additional input.
/// Since `Library` and `CloseableLibrary` still require at least one library name, so a dummy
/// value must be used.
///
/// # Windows Platform
///
/// The windows implementation must specify, which libraries the `SelfLoader` shall attempt to load from.
///
/// # Example
///
/// ```rust
/// use dylink::*;
/// use std::ffi::*;
///
/// static THIS: Library<SelfLoader> = Library::new(&[""]);
///
/// #[dylink(library=THIS)]
/// extern "C" {
/// 	fn atoi(s: *const c_char) -> c_int;
/// }
///
/// # #[cfg(unix)] {
/// let five = unsafe { atoi(b"5\0".as_ptr().cast()) };
/// assert_eq!(five, 5);
/// # }
/// ```
#[cfg(any(windows, unix, doc))]
pub struct SelfLoader(*mut core::ffi::c_void);
