// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use crate::*;
use std::ffi;
use std::io;


#[cfg(any(windows, unix, doc))]
mod self_loader;
#[cfg(any(windows, unix, doc))]
mod sys_loader;

#[doc(hidden)]
pub trait FnPtr: Copy + Clone {}
impl<T: Copy + Clone> FnPtr for T {}

#[cfg(any(feature = "close", doc))]
pub trait Closeable {
	unsafe fn close(self) -> io::Result<()>;
}


/// Used to specify the run-time linker loader constraint for [Library]
pub trait Loader: Send {
	fn is_invalid(&self) -> bool;
	unsafe fn load_library(lib_name: &'static ffi::CStr) -> Self;
	unsafe fn find_symbol(&self, fn_name: &'static ffi::CStr) -> FnAddr;
}

#[cfg(any(windows, unix, doc))]
pub struct SysLoader(*mut core::ffi::c_void);


/// `SelfLoader` is a special structure that retrieves symbols from libraries already
/// loaded before hand such as `libc` or `kernel32`
///
/// # Example
///
/// ```rust
/// use dylink::*;
/// use std::ffi::{c_char, c_int, CStr};
///
/// static LIBC_LIB: Library<SelfLoader, 1> = Library::new([
///   // dummy value for Library
///   unsafe { CStr::from_bytes_with_nul_unchecked(b"libc\0") }
/// ]);
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
