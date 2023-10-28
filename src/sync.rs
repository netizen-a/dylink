// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use std::{io, sync};

use crate::{Library, Symbol};

/// An object providing access to a lazily loaded LibLock on the filesystem.
///
/// This object is designed to be used with [`dylink`](crate::dylink).
#[derive(Debug)]
pub struct LibLock<'a> {
	libs: &'a [&'a str],
	// LibLock handle
	hlib: sync::OnceLock<Library>,
}

impl<'a> LibLock<'a> {
	/// Constructs a new `LibLock`.
	///
	/// This function accepts a slice of paths the LibLock will attempt to load from
	/// by priority (where `0..n`, index `0` is highest, and `n` is lowest), but only the first
	/// LibLock successfully loaded will be used. The reason is to provide fallback
	/// mechanism in case the shared LibLock is in a seperate directory or may have a variety
	/// of names.
	///
	/// If `libs` is empty then the program attempts to load itself.
	///
	/// # Examples
	///
	/// ```rust
	/// # use dylink::*;
	/// static KERNEL32: sync::LibLock = sync::LibLock::new(&["kernel32.dll"]);
	/// ```
	#[inline]
	pub const fn new(libs: &'a [&'a str]) -> Self {
		Self {
			libs,
			hlib: sync::OnceLock::new(),
		}
	}

	/// May block if another thread is currently attempting to initialize the cell.
	///
	/// This will lazily initialize the LibLock.
	///
	/// # Errors
	///
	/// If [`LibLock`] failed to be initialized, then this call will return an error.
	///
	/// If the requested symbol does not exist in the dynamic library, then this call will return an error.
	///
	/// # Examples
	///
	/// ```no_run
	/// use dylink::*;
	/// use std::mem;
	///
	/// let kernel32 = sync::LibLock::new(&["foo.dll"]);
	/// let sym = kernel32.symbol("my_symbol").unwrap();
	/// let my_symbol: unsafe extern "C" fn() = unsafe {mem::transmute(sym.cast::<()>())};
	/// ```
	pub fn symbol(&self, name: &str) -> io::Result<Symbol> {
		// yes this is cursed, no I can't do anything about it until get_or_try_init hits stable.
		// Fundamentally this function is designed to error and not panic hence catch_unwind.
		let lib = std::panic::catch_unwind(|| {
			self.hlib.get_or_init(|| {
				if self.libs.is_empty() {
					Library::this()
				} else {
					self.libs
						.iter()
						.find_map(|path| Library::open(path).ok())
						.unwrap()
				}
			})
		});
		match lib {
			Ok(lib) => lib.symbol(name),
			Err(_) => Err(io::Error::new(
				io::ErrorKind::Other,
				"Library initialization failed",
			)),
		}
	}
	/// Gets the reference to the underlying value.
	///
	/// Returns `None` if the cell is empty, or being initialized. This
	/// method never blocks.
	#[cfg(feature = "unstable")]
	#[inline]
	pub fn get(&self) -> Option<&Library> {
		self.hlib.get()
	}
	/// Takes the value out of this `LibLock`, moving it back to an uninitialized state.
	///
	/// Has no effect and returns `None` if the `LibLock` hasn't been initialized.
	///
	/// Safety is guaranteed by requiring a mutable reference.
	#[cfg(feature = "unstable")]
	#[inline]
	pub fn take(&mut self) -> Option<Library> {
		self.hlib.take()
	}

	#[cfg(feature = "unstable")]
	#[inline]
	pub fn set(&self, value: Library) -> Result<(), Library> {
		self.hlib.set(value)
	}

	#[cfg(feature = "unstable")]
	#[inline]
	pub fn into_inner(self) -> Option<Library> {
		self.hlib.into_inner()
	}
}
