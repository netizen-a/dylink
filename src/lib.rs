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
//! extern "system-unwind" {
//!     fn GetLastError() -> u32;
//!     fn SetLastError(_: u32);
//! }
//! ```

mod sealed;
use crate::sealed::Sealed;

pub mod os;
pub mod sync;

use core::ffi;
use std::{fs, io, marker, path, mem, ptr};

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
/// extern "system-unwind" {
///     fn foo();
/// }
///
/// // foreign function pattern
/// #[dylink(library=FOOBAR)]
/// extern "system-unwind" fn bar();
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
	#[cfg(feature="unstable")]
	/// Attempts to get base address of library.
	pub fn base_address(&self) -> io::Result<*mut std::ffi::c_void> {
		unsafe { imp::base_addr(self) }
	}
}

/// An object providing access to an open dynamic library.
///
/// Dynamic libraries are automatically dereferenced when they go out of scope.
/// Errors detected on closing are ignored by the implementation of `Drop`.
///
/// Threads executed by the dll must be terminated before the Library can be freed
/// or a race condition may occur.
///
/// The `Library` type is guarenteed access to the null pointer optimization.
#[derive(Debug)]
#[repr(transparent)]
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
	/// # Panics
	///
	/// May panic if library process handle could not be acquired.
	pub fn this() -> Self {
		unsafe { imp::dylib_this() }
			.map(Library)
			.expect("failed to acquire library process handle")
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
	pub fn path(&self) -> io::Result<path::PathBuf> {
		unsafe { imp::dylib_path(self.0) }
	}

	/// Queries metadata about the underlying library file.
	///
	/// This function is equivalent to calling `metadata` using `Library::path`.
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
	/// Creates two handles for a file named `foo.dll`:
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
		// windows has a more direct implementation of cloning here.
		#[cfg(windows)] unsafe {
			let handle = imp::dylib_clone(self.0)?;
			Ok(Library(handle))
		}
		// unix uses indirect cloning and may fail if path fails.
		// if there is a better way to do this on unix I'd like to know.
		#[cfg(not(windows))] {
			self.path().and_then(Library::open)
		}
	}
}

impl Drop for Library {
	/// Drops the Library.
	///
	/// This will decrement the reference count.
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

// weak may be null, but library may not be.

/// `Weak` is a version of [`Library`] that holds a non-owning reference to the dynamic library.
#[derive(Clone, Copy)]
pub struct Weak<'a>(*mut ffi::c_void, marker::PhantomData<&'a ()>);
unsafe impl Send for Weak<'_> {}
unsafe impl Sync for Weak<'_> {}

impl Weak<'_> {
	#[cfg(feature = "unstable")]
	pub fn try_default() -> io::Result<Weak<'static>> {
		let handle: *mut ffi::c_void = if cfg!(windows) {
			let s = std::ffi::OsString::from("kernel32.dll");
			unsafe {imp::dylib_open(&s)?}.as_ptr()
		} else {
			// RTLD_DEFAULT
			ptr::null_mut()
		};
		Ok(Weak(handle, marker::PhantomData))
	}
	pub fn upgrade(&self) -> Option<Library> {
		if let Some(p) = ptr::NonNull::new(self.0) {
			mem::ManuallyDrop::new(Library(p)).try_clone().ok()
		} else {
			None
		}
	}
}