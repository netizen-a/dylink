// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use crate::loader::Loader;
use crate::FnAddr;
use std::ffi::CStr;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Mutex;

#[cfg(feature = "unload")]
use crate::loader::Unloadable;

// this wrapper struct is the bane of my existance...
#[derive(Debug)]
pub(crate) struct FnAddrWrapper(pub FnAddr);
unsafe impl Send for FnAddrWrapper {}

#[derive(Debug)]
pub struct LazyLib<L: Loader, const N: usize> {
	libs: [&'static CStr; N],
	// library handle
	pub(crate) hlib: Mutex<Option<L>>,
}

impl<L: Loader, const N: usize> LazyLib<L, N> {
	/// # Panic
	/// Will panic if `libs` is an empty array.
	pub const fn new(libs: [&'static CStr; N]) -> Self {
		assert!(N > 0, "`libs` array cannot be empty.");
		Self {
			libs,
			hlib: Mutex::new(None),
		}
	}
	
	pub fn swap_sym(
		&self,
		sym_name: &'static CStr,
		atom: &AtomicPtr<()>,
		order: Ordering,
	) -> Option<*const ()> {
		let mut lock = self.hlib.lock().unwrap();
		if let None = *lock {
			for lib_name in self.libs {
				let handle = L::load_lib(lib_name);
				if !handle.is_invalid() {
					*lock = Some(handle);
					break;
				}
			}
		}

		if let Some(ref lib_handle) = *lock {
			let sym = L::load_sym(&lib_handle, sym_name);
			if sym.is_null() {
				None
			} else {
				Some(atom.swap(sym.cast_mut(), order))
			}
		} else {
			None
		}
	}
}

#[cfg(feature = "unload")]
pub struct UnloadableLazyLib<L: Loader + Unloadable, const N: usize> {
	inner: LazyLib<L, N>,
	reset_vec: Mutex<Vec<(&'static AtomicPtr<()>, FnAddrWrapper)>>,
}
#[cfg(feature = "unload")]
impl <L: Loader + Unloadable, const N: usize> UnloadableLazyLib<L, N> {
	/// # Panic
	/// Will panic if `libs` is an empty array.
	pub const fn new(libs: [&'static CStr; N]) -> Self {
		assert!(N > 0, "`libs` array cannot be empty.");
		Self {
			inner: LazyLib::new(libs),
			reset_vec: Mutex::new(Vec::new()),
		}
	}
	// finds symbol, loads library if not loaded, and does an atomic swap
	pub fn swap_sym(
		&self,
		sym_name: &'static CStr,
		atom: &'static AtomicPtr<()>,
		order: Ordering,
	) -> Option<*const ()> {
		match self.inner.swap_sym(sym_name, atom, order) {
			None => None,
			Some(function) => {
				self.reset_vec
					.lock()
					.unwrap()
					.push((atom, FnAddrWrapper(function)));
				Some(function)
			}
		}
	}

	/// Unloads the library and resets all associated function pointers to uninitialized state.
	///
	/// # Errors
	/// This may error if library is uninitialized.
	pub unsafe fn unload(&self) -> Result<(), ()> {
		let lock = self.inner.hlib.lock().unwrap();
		
		if let Some(ref handle) = *lock {
			let mut rstv_lock = self.reset_vec.lock().unwrap();
			for (pfn, FnAddrWrapper(init_pfn)) in rstv_lock.drain(..) {
				pfn.store(init_pfn.cast_mut(), Ordering::Release);
			}
			drop(rstv_lock);
			// decrement reference count on lib handle
			match handle.unload() {
				Ok(()) => Ok(()),
				Err(_) => Err(()),
			}
		} else {
			Err(())
		}
	}
}