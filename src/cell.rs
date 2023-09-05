// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use std::cell;
use crate::load;

/// An object providing access to a lazily loaded library on the filesystem.
///
/// This object is designed to be used with [`dylink`](crate::dylink) for subsequent zero overhead calls.
#[derive(Debug)]
pub struct Library<'a, L: load::Loader = load::System> {
    // NOTE: might make this mutable
	libs: &'a [&'a str],
	// library handle
	hlib: cell::OnceCell<L>,
}

impl<'a, L: load::Loader> Library<'a, L> {
	/// Constructs a new `Library`.
	///
	/// This function accepts a slice of paths the Library will attempt to load from
	/// by priority (where `0..n`, index `0` is highest, and `n` is lowest), but only the first
	/// library successfully loaded will be used. The reason is to provide fallback
	/// mechanism in case the shared library is in a seperate directory or may have a variety
	/// of names.
	///
	/// *Note: If `libs` is empty, the library cannot load.*
	///
	/// # Examples
	/// ```rust
	/// # use dylink::*;
	/// static KERNEL32: sync::Library<load::This> = sync::Library::new(&["kernel32.dll"]);
	/// ```
	pub const fn new(libs: &'a [&'a str]) -> Self {
		Self {
			libs,
			hlib: cell::OnceCell::new(),
		}
	}

	/// May block if another thread is currently attempting to initialize the cell.
	///
	/// This will lazily initialize the library.
	/// # Panics
	/// May panic if [`Library`] failed to be initialized.
	pub fn find(&self, symbol: &str) -> *const () {
		let handle = self.hlib.get_or_init(||{
			self.libs
				.iter()
				.find_map(|name| unsafe { L::open(name).ok() })
				.expect("failed to initialize `Library`")
		});
		unsafe { handle.sym(symbol) }
	}
	/// Gets the reference to the underlying value.
    ///
    /// Returns `None` if the cell is empty, or being initialized. This
    /// method never blocks.
	#[inline]
	pub fn get(&self) -> Option<&L> {
		self.hlib.get()
	}

    #[inline]
	pub fn into_inner(self) -> Option<L> {
		self.hlib.into_inner()
	}

	/// Takes the value out of this `Library`, moving it back to an uninitialized state.
    ///
    /// Has no effect and returns `None` if the `Library` hasn't been initialized.
    ///
    /// Safety is guaranteed by requiring a mutable reference.
	#[inline]
	pub fn take(&mut self) -> Option<L> {
		self.hlib.take()
	}

	#[inline]
	pub fn set(&self, value: L) -> Result<(), L> {
		self.hlib.set(value)
	}
}

#[cfg(any(unix, doc))]
impl Default for Library<'_, load::This> {
	fn default() -> Self {
		Self::new(&[""])
	}
}