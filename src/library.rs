// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use crate::loader::Loader;
use crate::FnAddr;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Mutex;
use std::io;


use crate::loader::Close;

// this wrapper struct is the bane of my existance...
#[derive(Debug)]
struct FnAddrWrapper(FnAddr);
unsafe impl Send for FnAddrWrapper {}

mod sealed {
	use super::*;
	pub trait Sealed {}
	impl <L: Loader> Sealed for Library<'_, L> {}
	impl <L: Loader + Close> Sealed for CloseableLibrary<'_, L> {}
}

/// Implements constraint to use the [`dylink`](crate::dylink) attribute macro `library` parameter.
pub trait FindAndSwap<'a>: sealed::Sealed {
	// I'd prefer if this made locking explicit, but then I'd need 2-4 structures for a sane API.
	/// Finds the address for `sym`, and returns the last address in `ppfn`.
	fn find_and_swap(
		&self,
		sym: &str,
		ppfn: &'a AtomicPtr<()>,
		order: Ordering,
	) -> Option<FnAddr>;
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
	/// # Panic
	/// Will panic if `libs` is an empty array.
	pub const fn new(libs: &'a [&'a str]) -> Self {
		assert!(!libs.is_empty());
		Self {
			libs,
			hlib: Mutex::new(None),
		}
	}
}

impl <'a, L: Loader> FindAndSwap<'a> for Library<'a, L> {
	/// Acquires a lock to load the library if not already loaded.
	/// Finds and stores a symbol into the `atom` pointer, returning the previous value.
	///
	/// `find_and_swap` takes an `Ordering` argument which describes the memory ordering of this operation. All ordering modes are possible. Note that using `Acquire` makes the store part of this operation `Relaxed`, and using `Release` makes the load part `Relaxed`.
	///
	/// Note: This method is only available on platforms that support atomic operations on pointers.
	fn find_and_swap(
		&self,
		sym: &str,
		ppfn: &AtomicPtr<()>,
		order: Ordering,
	) -> Option<FnAddr> {
		let mut lock = self.hlib.lock().unwrap();
		if let None = *lock {
			for lib_name in self.libs {
				let handle = unsafe {L::load_library(lib_name)};
				if !handle.is_invalid() {
					*lock = Some(handle);
					break;
				}
			}
		}

		if let Some(ref lib_handle) = *lock {
			let sym = unsafe {L::find_symbol(lib_handle, sym)};
			if sym.is_null() {
				None
			} else {
				Some(ppfn.swap(sym.cast_mut(), order))
			}
		} else {
			None
		}
	}
}


pub struct CloseableLibrary<'a, L: Loader + Close> {
	inner: Library<'a, L>,
	reset_vec: Mutex<Vec<(&'static AtomicPtr<()>, FnAddrWrapper)>>,
}

impl <'a, L: Loader + Close> CloseableLibrary<'a, L> {
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
			inner: Library::new(libs),
			reset_vec: Mutex::new(Vec::new()),
		}
	}

	/// closes the library and resets all associated function pointers to uninitialized state.
	///
	/// # Errors
	/// This may error if library is uninitialized.
	pub unsafe fn close(&self) -> io::Result<()> {
		if let Some(handle) = self.inner.hlib.lock().unwrap().take() {
			let mut rstv_lock = self.reset_vec.lock().unwrap();
			for (pfn, FnAddrWrapper(init_pfn)) in rstv_lock.drain(..) {
				pfn.store(init_pfn.cast_mut(), Ordering::Release);
			}
			drop(rstv_lock);
			match unsafe {handle.close()} {
				Ok(()) => Ok(()),
				Err(e) => Err(e),
			}
		} else {
			Err(io::Error::new(io::ErrorKind::InvalidInput, "`CloseableLibrary` is uninitialized."))
		}
	}
}

impl <L: Loader + Close> FindAndSwap<'static> for CloseableLibrary<'_, L> {
	fn find_and_swap(
		&self,
		sym: &str,
		ppfn: &'static AtomicPtr<()>,
		order: Ordering,
	) -> Option<FnAddr> {
		match self.inner.find_and_swap(sym, ppfn, order) {
			None => None,
			Some(function) => {
				self.reset_vec
					.lock()
					.unwrap()
					.push((ppfn, FnAddrWrapper(function)));
				Some(function)
			}
		}
	}
}