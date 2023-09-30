// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

#[cfg(unix)]
use crate::os::unix::dylib_symbol;
#[cfg(windows)]
use crate::os::windows::dylib_symbol;
use crate::{Library, Sym};
use std::cell;
use std::io;

/// An object providing access to a lazily loaded LibCell on the filesystem.
#[derive(Debug)]
pub struct LibCell<'a> {
	libs: &'a [&'a str],
	// LibCell handle
	hlib: cell::OnceCell<Library>,
}

impl<'a> LibCell<'a> {
	/// Constructs a new `LibCell`.
	///
	/// This function accepts a slice of paths the LibCell will attempt to load from
	/// by priority (where `0..n`, index `0` is highest, and `n` is lowest), but only the first
	/// LibCell successfully loaded will be used. The reason is to provide fallback
	/// mechanism in case the shared LibCell is in a seperate directory or may have a variety
	/// of names.
	///
	/// If `libs` is empty then the program attempts to load itself.
	///
	/// # Examples
	/// ```rust
	/// # use dylink::*;
	/// let _kernel32: cell::LibCell = cell::LibCell::new(&["kernel32.dll"]);
	/// ```
	pub const fn new(libs: &'a [&'a str]) -> Self {
		Self {
			libs,
			hlib: cell::OnceCell::new(),
		}
	}

	/// May block if another thread is currently attempting to initialize the cell.
	///
	/// This will lazily initialize the LibCell.
	/// # Panics
	/// May panic if [`LibCell`] failed to be initialized.
	pub fn symbol(&'a self, name: &'a str) -> io::Result<&'a Sym> {
		let lib = self.hlib.get_or_init(|| {
			if self.libs.is_empty() {
				Library::this().expect("failed to initialize `LibLock`")
			} else {
				self.libs
					.iter()
					.find_map(|path| Library::open(path).ok())
					.expect("failed to initialize `LibLock`")
			}
		});
		unsafe {
			// ValidHandle::as_ptr is safe here, because we got the
			// library through OnceCell::get_or_init
			dylib_symbol(*lib.0.as_ptr(), name)
		}
	}
	/// Gets the reference to the underlying value.
	///
	/// Returns `None` if the cell is empty, or being initialized. This
	/// method never blocks.
	#[inline]
	pub fn get(&self) -> Option<&Library> {
		self.hlib.get()
	}

	/// Takes the value out of this `LibCell`, moving it back to an uninitialized state.
	///
	/// Has no effect and returns `None` if the `LibCell` hasn't been initialized.
	///
	/// Safety is guaranteed by requiring a mutable reference.
	#[inline]
	pub fn take(&mut self) -> Option<Library> {
		self.hlib.take()
	}

	#[inline]
	pub fn set(&self, value: Library) -> Result<(), Library> {
		self.hlib.set(value)
	}

	#[inline]
	pub fn into_inner(self) -> Option<Library> {
		self.hlib.into_inner()
	}
}
