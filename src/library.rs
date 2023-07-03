// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use crate::loader::Loader;
use crate::FnAddr;
use std::ffi::CStr;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Mutex;
use std::io;

#[cfg(feature = "close")]
use crate::loader::Closeable;

// this wrapper struct is the bane of my existance...
#[derive(Debug)]
struct FnAddrWrapper(pub FnAddr);
unsafe impl Send for FnAddrWrapper {}

mod sealed {
	use super::*;
	pub trait Sealed {}
	impl <L: Loader, const N: usize> Sealed for Library<L, N> {}
	impl <L: Loader + Closeable, const N: usize> Sealed for CloseableLibrary<L, N> {}
}

/// Implements constraint to use the [`dylink`](crate::dylink) attribute macro `library` parameter.
pub trait FindAndSwap<'a>: sealed::Sealed {
	// I'd prefer if this made locking explicit, but then I'd need 2-4 structures for a sane API.
	/// Finds the address for `sym`, and returns the last address in `ppfn`.
	fn find_and_swap(
		&self,
		sym: &'static CStr,
		ppfn: &'a AtomicPtr<()>,
		order: Ordering,
	) -> Option<*const ()>;
}

/// A library handle.
/// 
/// 
#[derive(Debug)]
pub struct Library<L: Loader, const N: usize> {
	libs: [&'static CStr; N],
	// library handle
	hlib: Mutex<Option<L>>,
}

unsafe impl<L: Loader + Send, const N: usize> Send for Library<L, N> {}
unsafe impl<L: Loader + Send, const N: usize> Sync for Library<L, N> {}

impl<L: Loader, const N: usize> Library<L, N> {
	/// Constructs a new `Library`.
	/// 
	/// # Panic
	/// Will panic if `libs` is an empty array.
	pub const fn new(libs: [&'static CStr; N]) -> Self {
		assert!(N > 0, "`libs` array cannot be empty.");
		Self {
			libs,
			hlib: Mutex::new(None),
		}
	}
}

#[cfg(target_has_atomic = "ptr")]
impl <'a, L: Loader, const N: usize> FindAndSwap<'a> for Library<L, N> {
	/// Acquires a lock to load the library if not already loaded.
	/// Finds and stores a symbol into the `atom` pointer, returning the previous value.
	/// 
	/// `find_and_swap` takes an `Ordering` argument which describes the memory ordering of this operation. All ordering modes are possible. Note that using `Acquire` makes the store part of this operation `Relaxed`, and using `Release` makes the load part `Relaxed`.
	/// 
	/// Note: This method is only available on platforms that support atomic operations on pointers.
	fn find_and_swap(
		&self,
		sym: &'static CStr,
		ppfn: &AtomicPtr<()>,
		order: Ordering,
	) -> Option<*const ()> {
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

#[cfg(feature = "close")]
pub struct CloseableLibrary<L: Loader + Closeable, const N: usize> {
	inner: Library<L, N>,
	reset_vec: Mutex<Vec<(&'static AtomicPtr<()>, FnAddrWrapper)>>,
}

#[cfg(feature = "close")]
unsafe impl<L: Loader + Closeable + Send, const N: usize> Send for CloseableLibrary<L, N> {}
#[cfg(feature = "close")]
unsafe impl<L: Loader + Closeable + Send, const N: usize> Sync for CloseableLibrary<L, N> {}

#[cfg(feature = "close")]
impl <L: Loader + Closeable, const N: usize> CloseableLibrary<L, N> {
	/// # Panic
	/// Will panic if `libs` is an empty array.
	pub const fn new(libs: [&'static CStr; N]) -> Self {
		assert!(N > 0, "`libs` array cannot be empty.");
		Self {
			inner: Library::new(libs),
			reset_vec: Mutex::new(Vec::new()),
		}
	}

	/// closes the library and resets all associated function pointers to uninitialized state.
	///
	/// # Errors
	/// This may error if library is uninitialized.
	pub fn close(&self) -> io::Result<()> {		
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

impl <'a, L: Loader + Closeable, const N: usize> FindAndSwap<'static> for CloseableLibrary<L, N> {
	fn find_and_swap(
		&self,
		sym: &'static CStr,
		ppfn: &'static AtomicPtr<()>,
		order: Ordering,
	) -> Option<*const ()> {
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