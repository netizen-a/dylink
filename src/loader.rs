// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use crate::*;
use std::io;


#[cfg(any(windows, unix, doc))]
mod self_loader;
#[cfg(any(windows, unix, doc))]
mod sys_loader;

/// This trait is similar to the `Drop` trait, which frees resources.
/// Unlike the `Drop` trait, `Close` must assume there side affects when closing a library.
/// As a consequence of these side affects `close` is marked as `unsafe`.
///
/// This trait should not be used directly, and instead be used in conjunction with `CloseableLibrary`,
/// so that the lifetimes of retrieved symbols are not invalidated.

pub unsafe trait Close {
	unsafe fn close(self) -> io::Result<()>;
}

/// Used to specify the run-time linker loader constraint for [`Library`]
pub unsafe trait Loader: Send {
	fn is_invalid(&self) -> bool;
	unsafe fn load_library(path: &str) -> Self;
	unsafe fn find_symbol(&self, symbol: &str) -> FnAddr;
}

/// A system library loader.
///
/// This is a basic library loader primitive designed to be used with [`Library`].
#[cfg(any(windows, unix, doc))]
pub struct SystemLoader(*mut core::ffi::c_void);


/// A retroactive system loader.
///
/// This loader is responsible for retrieving symbols from libraries already loaded.
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
/// use std::ffi::{c_char, c_int, CStr};
///
/// static LIBC_LIB: Library<SelfLoader> = Library::new(&["libc"]);
///
/// #[dylink(library=LIBC_LIB)]
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
