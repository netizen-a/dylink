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
	/// # Panics
	///
	/// panics if `libs` is empty.
	///
	/// # Examples
	/// ```rust
	/// # use dylink::*;
	/// static KERNEL32: sync::LibLock = sync::LibLock::new(&["kernel32.dll"]);
	/// ```
	#[inline]
	pub const fn new(libs: &'a [&'a str]) -> Self {
		assert!(!libs.is_empty(), "`libs` cannot be empty");
		Self {
			libs,
			hlib: sync::OnceLock::new(),
		}
	}

	/// May block if another thread is currently attempting to initialize the cell.
	///
	/// This will lazily initialize the LibLock.
	/// # Panics
	/// May panic if [`LibLock`] failed to be initialized.
	pub fn symbol(&'a self, name: &str) -> io::Result<Symbol> {
		let lib = self.hlib.get_or_init(|| {
			self.libs
				.iter()
				.find_map(|path| Library::open(path).ok())
				.expect("failed to initialize `LibLock`")
		});
		lib.symbol(name)
	}
	/// Gets the reference to the underlying value.
	///
	/// Returns `None` if the cell is empty, or being initialized. This
	/// method never blocks.
	#[cfg(feature="unstable")]
	#[inline]
	pub fn get(&self) -> Option<&Library> {
		self.hlib.get()
	}
	/// Takes the value out of this `LibLock`, moving it back to an uninitialized state.
	///
	/// Has no effect and returns `None` if the `LibLock` hasn't been initialized.
	///
	/// Safety is guaranteed by requiring a mutable reference.
	#[cfg(feature="unstable")]
	#[inline]
	pub fn take(&mut self) -> Option<Library> {
		self.hlib.take()
	}

	#[cfg(feature="unstable")]
	#[inline]
	pub fn set(&self, value: Library) -> Result<(), Library> {
		self.hlib.set(value)
	}

	#[cfg(feature="unstable")]
	#[inline]
	pub fn into_inner(self) -> Option<Library> {
		self.hlib.into_inner()
	}
}
