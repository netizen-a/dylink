// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
#![cfg_attr(docsrs, feature(doc_auto_cfg), feature(doc_cfg))]

//! Dylink provides a run-time dynamic linking framework for loading dynamic libraries.
//! You can load libraries directly through [`Library`], which enables diverse error handling,
//! or you can load libraries indirectly through [`LibLock`](crate::sync::LibLock) and `dylink`.
//!
//! # Platform support
//! Platform support typically varies between functions, however unless otherwise specified, functions
//! are supported on Windows, Linux, and MacOS.

mod sealed;
use crate::sealed::Sealed;

pub mod os;
#[cfg(unix)]
use os::unix as imp;
#[cfg(windows)]
use os::windows as imp;

pub mod iter;
pub mod sync;
mod weak;
pub use weak::Weak;

use std::{io, marker, path};

pub use dylink_macro::dylink;

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
	/// Attempts to get the base address of the library.
	///
	/// # Platform support
	///
	/// This function is supported on all platforms unconditionally, and should be
	/// preferred over [`Image::as_ptr`] when possible.
	#[inline]
	pub fn base_address(&self) -> io::Result<*mut os::Header> {
		unsafe { imp::base_addr(self.0) }
	}
}

/// An object providing access to an open dynamic library.
///
/// Errors detected on closing are ignored by the implementation of `Drop`.
///
/// # Safety
///
/// Threads executed by a dynamic library must be terminated before the Library can be freed
/// or a race condition may occur. Additionally, upon loading or unloading the library, an
/// optional entry point may be executed for each library, which may impose arbitrary requirements on the
/// user for the access to the library to be sound.
#[derive(Debug)]
#[repr(transparent)]
pub struct Library(os::Handle);
unsafe impl Send for Library {}
unsafe impl Sync for Library {}
impl crate::sealed::Sealed for Library {}

impl Library {
	/// Attempts to open a dynamic library file.
	///
	/// The library maintains an internal reference count that increments
	/// for every time the library is opened. Library symbols are eagerly resolved
	/// before the function returns.
	///
	/// # Examples
	///
	/// ```no_run
	/// use dylink::Library;
	///
	/// let lib = Library::open("foo.dll").unwrap();
	/// ```
	#[doc(alias = "dlopen", alias = "LoadLibrary")]
	#[inline]
	pub fn open<P: AsRef<path::Path>>(path: P) -> io::Result<Self> {
		unsafe { imp::dylib_open(path.as_ref().as_os_str()) }.map(Library)
	}
	/// Attempts to returns a library handle to the current process.
	///
	/// # Panics
	///
	/// May panic if library process handle could not be acquired.
	///
	/// # Examples
	///
	/// ```
	/// use dylink::{Library, Image};
	///
	/// let this = Library::this();
	/// let path = this.path().unwrap();
	/// println!("{}", path.display());
	/// ```
	#[must_use]
	#[inline]
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
	///
	/// # Examples
	///
	/// ```no_run
	/// # #[repr(transparent)]
	/// # struct Display(*const ffi::c_void);
	/// use std::{mem, ffi};
	/// use dylink::Library;
	///
	/// type PfnXOpenDisplay = extern "C-unwind" fn (display_name: *const ffi::c_char) -> *mut Display;
	///
	/// let lib = Library::open("libX11.so.6").unwrap();
	/// let sym = lib.symbol("XOpenDisplay").unwrap();
	/// let xopendisplay: PfnXOpenDisplay = unsafe {mem::transmute(sym.cast::<()>())};
	/// ```
	#[doc(alias = "dlsym")]
	#[inline]
	pub fn symbol<'a>(&'a self, name: &str) -> io::Result<Symbol<'a>> {
		unsafe { imp::dylib_symbol(self.0.as_ptr(), name) }
	}
	#[cfg(feature="unstable")]
	#[doc(alias = "dlsym")]
	#[inline]
	pub fn symbol_cstr(&self, name: &std::ffi::CStr) -> *const std::ffi::c_void {
		unsafe {imp::dylib_c_symbol(self.0.as_ptr(), name)}
	}

	/// Creates a new `Library` instance that shares the same underlying library handle as the
	/// existing `Library` instance.
	///
	/// # Examples
	///
	/// Creates two handles for a library named `foo.dll`:
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
	#[inline]
	pub fn try_clone(&self) -> io::Result<Library> {
		let handle = unsafe { imp::dylib_clone(self.0)? };
		Ok(Library(handle))
	}

	/// Creates a new [`Weak`] pointer to this Library.
	///
	/// # Examples
	///
	/// ```no_run
	/// use dylink::Library;
	///
	/// fn main() -> std::io::Result<()> {
	///     let lib = Library::open("foo.dll")?;
	///     let weak_lib = Library::downgrade(&lib);
	///     Ok(())
	/// }
	/// ```
	pub fn downgrade(this: &Self) -> weak::Weak {
		weak::Weak {
			base_addr: Image::as_ptr(this),
			path_name: Image::path(this).ok(),
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

impl Image for Library {
	fn as_ptr(&self) -> *const os::Header {
		unsafe { imp::get_addr(self.0) }
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
	///
	/// # Examples
	///
	/// ```no_run
	/// use dylink::{Library, Image};
	///
	/// fn main() -> std::io::Result<()> {
	///     let mut lib = Library::open("foo.dll")?;
	///     let path = lib.path()?;
	///     Ok(())
	/// }
	/// ```
	#[doc(
		alias = "dlinfo",
		alias = "_dyld_get_image_name",
		alias = "GetModuleFileNameW"
	)]
	#[inline]
	fn path(&self) -> io::Result<path::PathBuf> {
		unsafe { imp::dylib_path(self.0) }
	}
}

/// Creates an `Option<Library>` that may contain a loaded library.
///
/// `lib!` allows `Library`s to be defined with the same syntax as an array expression.
/// ```rust
/// use dylink::*;
/// let lib: Option<Library> = lib!["libvulkan.dylib", "libvulkan.1.dylib", "libMoltenVK.dylib"];
/// ```
#[macro_export]
macro_rules! lib {
	($($name:expr),+ $(,)?) => {
		[$($name),+].into_iter()
			.find_map(|elem| $crate::Library::open(elem).ok())
	};
}

/// A trait for objects that represent executable images.
pub trait Image: crate::sealed::Sealed {
	/// Returns the base address of the image.
	///
	/// The pointer is only valid if there are some strong references to the image.
	/// The pointer may be dangling, unaligned or even [`null`] otherwise.
	///
	/// [`null`]: core::ptr::null "ptr::null"
	fn as_ptr(&self) -> *const os::Header;
	fn path(&self) -> io::Result<path::PathBuf>;
	/// Returns `true` if the two `Image`s point to the same base address in a vein similar to [`ptr::eq`].
	/// This function ignores metadata of `dyn Trait` pointers.
	///
	/// [`ptr::eq`]: core::ptr::eq "ptr::eq"
	fn ptr_eq(&self, other: &impl Image) -> bool {
		self.as_ptr() == other.as_ptr()
	}
}
