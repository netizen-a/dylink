// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use crate::loader::Loader;
use crate::SymAddr;
use std::io;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{LockResult, Mutex, MutexGuard, PoisonError};

use crate::loader::Close;

mod guard;


#[derive(Debug)]
pub struct LibraryGuard<'a, L: Loader> {
	libs: &'a [&'a str],
	guard: MutexGuard<'a, Option<L>>,
}


#[derive(Debug)]
pub struct CloseableLibraryGuard<'a, L: Loader> {
	libs: &'a [&'a str],
	guard: MutexGuard<'a, (Option<L>, Vec<(&'static AtomicPtr<()>, SymAddrWrapper)>)>,
}


// this wrapper struct is the bane of my existance...
#[derive(Debug)]
struct SymAddrWrapper(SymAddr);
unsafe impl Send for SymAddrWrapper {}

mod sealed {
	use super::*;
	pub trait Sealed {}
	impl<L: Loader> Sealed for Library<'_, L> {}
	impl<L: Close> Sealed for CloseableLibrary<'_, L> {}
}

/// Implements constraint to use the [`dylink`](crate::dylink) attribute macro `library` parameter.
pub trait LibraryLock<'a>: sealed::Sealed {
	type Guard: 'a;
	fn lock(&'a self) -> LockResult<Self::Guard>;
}

/// A library handle.
#[derive(Debug)]
pub struct Library<'a, L: Loader> {
	libs: &'a [&'a str],
	// library handle
	hlib: Mutex<Option<L>>,
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
	/// *Note: Symbols used in the libraries **must** be the same in all fallback paths.*
	///
	/// # Panics
	/// Will panic if `libs` is an empty array.
	///
	/// # Examples
	/// ```rust
	/// # use dylink::*;
	/// static KERNEL32: Library<SelfLoader> = Library::new(&["kernel32.dll"]);
	/// ```
	pub const fn new(libs: &'a [&'a str]) -> Self {
		assert!(!libs.is_empty());
		Self {
			libs,
			hlib: Mutex::new(None),
		}
	}
}

impl<'a, L: Loader + 'a> LibraryLock<'a> for Library<'a, L> {
	type Guard = LibraryGuard<'a, L>;
	/// acquires lock
	fn lock(&'a self) -> LockResult<Self::Guard> {
		self.hlib
			.lock()
			.map(|guard| LibraryGuard {
				libs: self.libs,
				guard,
			})
			.or_else(|poison| {
				Err(PoisonError::new(LibraryGuard {
					libs: self.libs,
					guard: poison.into_inner(),
				}))
			})
	}
}

pub struct CloseableLibrary<'a, L: Close> {
	libs: &'a [&'a str],
	inner: Mutex<(Option<L>, Vec<(&'static AtomicPtr<()>, SymAddrWrapper)>)>,
}

impl<'a, L: Close> CloseableLibrary<'a, L> {
	/// Constructs a new `CloseableLibrary`.
	///
	/// This function accepts a slice of paths the Library will attempt to load from
	/// by priority (where `0..n`, index `0` is highest, and `n` is lowest), but only the first
	/// library successfully loaded will be used. The reason is to provide fallback
	/// mechanism in case the shared library is in a seperate directory or may have a variety
	/// of names.
	///
	/// *Note: Symbols used in the libraries **must** be the same in all fallback paths.*
	///
	/// # Panic
	/// Will panic if `libs` is an empty array.
	pub const fn new(libs: &'a [&'a str]) -> Self {
		assert!(!libs.is_empty());
		Self {
			libs,
			inner: Mutex::new((None, Vec::new())),
		}
	}
}

impl<'a, L: Close + 'a> LibraryLock<'a> for CloseableLibrary<'a, L> {
	type Guard = CloseableLibraryGuard<'a, L>;
	/// acquires lock
	fn lock(&'a self) -> LockResult<Self::Guard> {
		self.inner
			.lock()
			.map(|guard| CloseableLibraryGuard {
				libs: self.libs,
				guard,
			})
			.or_else(|poison| {
				Err(PoisonError::new(CloseableLibraryGuard {
					libs: self.libs,
					guard: poison.into_inner(),
				}))
			})
	}
}
