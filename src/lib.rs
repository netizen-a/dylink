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

pub mod os;
#[cfg(unix)]
use os::unix as imp;
#[cfg(windows)]
use os::windows as imp;

pub mod img;
pub mod sync;

mod weak;
pub use weak::Weak;

mod sym;
pub use sym::Symbol;

use std::{io, path};

pub use dylink_macro::dylink;

#[doc = include_str!("../README.md")]
#[cfg(all(doctest, windows))]
struct ReadmeDoctests;



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
pub struct Library(imp::InnerLibrary);
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
	/// # Security
	///
	/// To prevent dynamic library [preloading attacks] its recommended to use a fully qualified path,
	/// or remove the current working directory from the list of search paths.
	///
	/// [preloading attacks]: https://learn.microsoft.com/en-us/windows/win32/dlls/dynamic-link-library-security
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
		unsafe { imp::InnerLibrary::open(path.as_ref().as_os_str()) }.map(Self)
	}
	/// Attempts to return a library handle to the current process.
	///
	/// # Panics
	///
	/// May panic if library process handle could not be acquired.
	///
	/// # Examples
	///
	/// ```
	/// use dylink::Library;
	///
	/// let this = Library::this();
	/// ```
	#[must_use]
	#[inline]
	pub fn this() -> Self {
		unsafe { imp::InnerLibrary::this() }
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
		unsafe { self.0.symbol(name) }
	}
	#[cfg(feature = "unstable")]
	#[doc(alias = "dlsym")]
	#[inline]
	pub fn c_symbol(&self, name: &std::ffi::CStr) -> *const std::ffi::c_void {
		unsafe { self.0.c_symbol(name) }
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
		unsafe { self.0.try_clone().map(Library) }
	}

	// May not be applicable to running process (Self::this), hence Option type.
	/// Converts this library to a header.
	///
	/// *Note: Whenever possible, [`Symbol::image`] should be preferred.*
	pub fn to_image(&self) -> io::Result<&img::Image> {
		unsafe { self.0.to_ptr().as_ref() }.ok_or(io::Error::new(
			io::ErrorKind::Unsupported,
			"Header cannot be retrieved on this platform. Use `Symbol::header` instead.",
		))
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
	pub fn downgrade(this: &Self) -> io::Result<weak::Weak> {
		let base_addr = this.to_image()?;
		Ok(weak::Weak {
			base_addr,
			path_name: base_addr.path().ok(),
		})
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
