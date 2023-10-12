// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! Dylink provides a run-time dynamic linking framework for lazily evaluating shared libraries.
//! When functions are loaded they are evaluated through a thunk for first time calls, which loads the function
//! from its respective library. Preceeding calls after initialization have no overhead or additional branching
//! checks, since the thunk is replaced by the loaded function.
//!
//! # Platform support
//! Platform support typically varies between functions, however unless otherwise specified, functions
//! are minimally supported on Windows, Linux, and MacOS.
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

mod sealed;
use crate::sealed::Sealed;

pub mod os;
pub mod sync;

use std::{fs, io, marker, mem, path};

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
use os::unix as imp;
#[cfg(windows)]
use os::windows as imp;

#[doc = include_str!("../README.md")]
#[cfg(all(doctest, windows))]
struct ReadmeDoctests;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(C)]
pub struct Symbol<'a>(*mut std::ffi::c_void, marker::PhantomData<&'a ()>);
impl Sealed for Symbol<'_> {}

impl Symbol<'_> {
	/// Casts to a pointer of another type.
	#[inline]
	pub const fn cast<T>(&self) -> *mut T {
		self.0 as _
	}
	/// Attempts to get base address of library.
	pub fn base_addr(&self) -> io::Result<*const std::ffi::c_void> {
		unsafe { imp::base_addr(self) }
	}
}

/// An object providing access to an open dynamic library.
///
/// The type `Library` provides a shared ownership to an open dynamic library.
#[derive(Debug)]
pub struct Library(os::Handle);
unsafe impl Send for Library {}
unsafe impl Sync for Library {}

impl Library {
	/// Attempts to open a dynamic library file.
	///
	/// The library maintains an internal reference count that increments
	/// for every time the library is opened. Library symbols are eagerly resolved
	/// before the function returns.
	///
	/// # Safety
	///
	/// Upon loading or unloading the library, an optional entry point may be executed
	/// for each library.
	#[doc(alias = "dlopen", alias = "LoadLibrary")]
	pub fn open<P: AsRef<path::Path>>(path: P) -> io::Result<Self> {
		unsafe { imp::dylib_open(path.as_ref().as_os_str()) }.map(Library)
	}
	/// Attempts to returns a library handle to the current process.
	///
	/// # Errors
	///
	/// May error if library process handle could not be acquired.
	pub fn this() -> io::Result<Self> {
		unsafe { imp::dylib_this() }
			.map(Library)
	}
	/// Same as drop, but returns a result.
	///
	/// This method is recommended when using other crates that manipulate dynamic libraries.
	///
	/// # Errors
	///
	/// May return an error if failed to close library.
	#[doc(alias = "dlclose")]
	pub fn close(self) -> io::Result<()> {
		unsafe { imp::dylib_close(mem::ManuallyDrop::new(self).0) }
	}

	/// Retrieves a symbol from the library if it exists
	///
	/// # Errors
	///
	/// May error if symbol is not found.
	#[doc(alias = "dlsym")]
	pub fn symbol<'a>(&'a self, name: &str) -> io::Result<Symbol<'a>> {
		unsafe { imp::dylib_symbol(self.0, name) }
	}

	/// Gets the path to the dynamic library file.
	///
	/// # Platform-specific behavior
	/// This function currently corresponds to the `dlinfo` function on Linux, `_dyld_get_image_name` on MacOS,
	/// and `GetModuleFileNameW` function on Windows. Note that, this [may change in the future][changes]
	///
	/// [changes]: io#platform-specific-behavior
	///
	/// *Note: This function is not guarenteed to return the same path as the one passed in to open the library.*
	///
	/// # Errors
	///
	/// This function will return an error if there is no path associated with the library handle.
	#[doc(
		alias = "dlinfo",
		alias = "_dyld_get_image_name",
		alias = "GetModuleFileNameW"
	)]
	#[cfg(any(windows, target_os="macos", target_env="gnu"))]
	pub fn path(&self) -> io::Result<path::PathBuf> {
		unsafe { imp::dylib_path(self.0) }
	}

	/// This is the preferred way to close libraries when exiting threads.
	pub fn close_and_exit(lib: Library, exit_code: i32) -> ! {
		unsafe { imp::dylib_close_and_exit(lib.0, exit_code) }
	}

	/// Queries metadata about the underlying library file.
	///
	/// # Examples
	///
	/// ```no_run
	/// use dylink::Library;
	///
	/// fn main() -> std::io::Result<()> {
	///     let mut lib = Library::open("foo.dll")?;
	///     let metadata = lib.metadata()?;
	///     Ok(())
	/// }
	/// ```
	#[cfg(any(windows, target_os="macos", target_env="gnu"))]
	pub fn metadata(&self) -> io::Result<fs::Metadata> {
		self.path().and_then(fs::metadata)
	}
	/// Creates a new `Library` instance that shares the same underlying library handle as the
	/// existing `Library` instance.
	///
	/// Creates two handles for a file named `foo.txt`:
	///
	/// ```no_run
	/// use dylink::Library;
	///
	/// fn main() -> std::io::Result<()> {
	///     let mut lib = Library::open("foo.dll")?;
	///     let lib_copy = lib.try_clone()?;
	///     Ok(())
	/// }
	/// ```
	#[cfg(any(windows, target_os="macos", target_env="gnu"))]
	pub fn try_clone(&self) -> io::Result<Library> {
		self.path().and_then(Library::open)
	}
}

impl Drop for Library {
	fn drop(&mut self) {
		unsafe {
			let _ = imp::dylib_close(self.0);
		}
	}
}

/// Creates an `Option<Library>` that may contain a loaded library.
///
/// `lib!` allows `Library`s to be defined with the same syntax as an array expression.
/// ```rust
/// use dylink::*;
/// let lib: Option<Library> = lib!["libX11.so.6", "Kernel32.dll", "libSystem.dylib"];
/// ```
#[macro_export]
macro_rules! lib {
	($($name:expr),+ $(,)?) => {
		[$($name),+].into_iter()
			.find_map(|elem| $crate::Library::open(elem).ok())
	};
}

#[cfg(feature = "unstable")]
#[cfg(any(
	windows,
	target_os = "linux",
	target_os = "macos",
	target_env = "gnu",
	docsrs
))]
pub fn is_loaded<P: AsRef<path::Path>>(path: P) -> bool {
	unsafe { imp::dylib_is_loaded(path.as_ref().as_os_str()) }
}