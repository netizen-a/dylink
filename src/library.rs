// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use crate::loader::Loader;
use crate::SymAddr;
use std::sync;

/// An object providing access to a lazily loaded library on the filesystem.
///
/// This object is designed to be used with [`dylink`](crate::dylink) for subsequent zero overhead calls.
#[derive(Debug)]
pub struct Library<'a, L: Loader> {
	libs: &'a [&'a str],
	// library handle
	hlib: sync::OnceLock<L>,
}

impl<'a, L: Loader> Library<'a, L> {
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
	/// static KERNEL32: Library<SelfLoader> = Library::new(&["kernel32.dll"]);
	/// ```
	pub const fn new(libs: &'a [&'a str]) -> Self {
		Self {
			libs,
			hlib: sync::OnceLock::new(),
		}
	}

	/// May block if another thread is currently attempting to initialize the cell.
	///
	/// This will lazily initialize the library.
	/// # Panics
	/// May panic if [`Library`] failed to be initialized.
	pub fn find_symbol(&self, symbol: &str) -> SymAddr {
		let handle = self.hlib.get_or_init(||{
			self.libs
				.iter()
				.find_map(|name| unsafe { L::open(name).ok() })
				.expect("failed to initialize `Library`")
		});
		unsafe { handle.find_symbol(symbol) }
	}
	#[inline]
	pub fn get(&self) -> Option<&L> {
		self.hlib.get()
	}
	#[inline]
	pub fn take(&mut self) -> Option<L> {
		self.hlib.take()
	}
}
