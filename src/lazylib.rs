// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

//use crate::loader::LibHandle;
use crate::loader::Loader;
use crate::{FnAddr, Unloadable};
use alloc::boxed::Box;
use core::ffi::CStr;
use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

#[cfg(feature = "unload")]
use std::sync::Mutex;

// this wrapper struct is the bane of my existance...
#[derive(Debug)]
pub(crate) struct FnAddrWrapper(pub FnAddr);
unsafe impl Send for FnAddrWrapper {}

#[derive(Debug)]
pub struct LazyLib<L: Loader, const N: usize> {
	// atomic lock
	pub(crate) atml: AtomicBool,
	libs: [&'static CStr; N],
	// library handle
	pub(crate) hlib: AtomicPtr<L>,
}

impl<L: Loader, const N: usize> LazyLib<L, N> {
	/// # Panic
	/// Will panic if `libs` is an empty array.
	pub const fn new(libs: [&'static CStr; N]) -> Self {
		assert!(N > 0, "`libs` array cannot be empty.");
		Self {
			atml: AtomicBool::new(false),
			libs,
			hlib: AtomicPtr::new(core::ptr::null_mut()),
		}
	}
	/// loads function from library synchronously and binds library handle internally to dylink.
	///
	/// If the library is already bound, the bound handle will be used for loading the function.
	pub unsafe fn find_sym(
		&self,
		sym_name: &'static CStr,
		atom: &AtomicPtr<()>,
	) -> Option<*const ()> {
		// lock
		while self.atml.swap(true, Ordering::Acquire) {
			#[cfg(feature = "std")]
			{
				// Not doing anything, so yield time to other threads.
				std::thread::yield_now()
			}
			#[cfg(not(feature = "std"))]
			{
				// `no_std` enviroments can't yield, so just use a busy wait.
				// This isn't costly since loading is not expected to take long.
				core::hint::spin_loop()
			}
		}

		if let None = self.hlib.load(Ordering::Acquire).as_ref() {
			for lib_name in self.libs {
				let handle = L::load_lib(lib_name);
				if !handle.is_invalid() {
					self.hlib
						.store(Box::into_raw(Box::new(handle)), Ordering::Release);
					break;
				}
			}
		}
		// unlock
		self.atml.store(false, Ordering::Release);

		if let Some(lib_handle) = self.hlib.load(Ordering::Acquire).as_ref() {
			let sym = L::load_sym(&lib_handle, sym_name);
			if sym.is_null() {
				None
			} else {
				atom.store(sym.cast_mut(), Ordering::Release);
				Some(sym)
			}
		} else {
			None
		}
	}
}

impl<L: Loader, const N: usize> Drop for LazyLib<L, N> {
	fn drop(&mut self) {
		let maybe_handle = self.hlib.load(Ordering::Relaxed);
		if !maybe_handle.is_null() {
			unsafe {
				drop(Box::from_raw(maybe_handle));
			}
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
	/// loads function from library synchronously and binds library handle internally to dylink.
	///
	/// If the library is already bound, the bound handle will be used for loading the function.
	pub unsafe fn find_sym(
		&self,
		sym_name: &'static CStr,
		atom: &'static AtomicPtr<()>,
	) -> Option<*const ()> {
		let init = atom.load(Ordering::Acquire);
		match self.inner.find_sym(sym_name, atom) {
			None => None,
			Some(function) => {
				self.reset_vec
					.lock()
					.unwrap()
					.push((atom, FnAddrWrapper(init)));
				Some(function)
			}
		}
	}

	/// Unloads the library and resets all associated function pointers to uninitialized state.
	///
	/// # Errors
	/// This may error if library is uninitialized.
	pub unsafe fn unload(&self) -> Result<(), ()> {
		// lock
		while self.inner.atml.swap(true, Ordering::Acquire) {
			core::hint::spin_loop()
		}

		let phandle = self.inner.hlib.swap(core::ptr::null_mut(), Ordering::SeqCst);
		if !phandle.is_null() {
			let mut rstv_lock = self.reset_vec.lock().unwrap();
			for (pfn, FnAddrWrapper(init_pfn)) in rstv_lock.drain(..) {
				pfn.store(init_pfn.cast_mut(), Ordering::Release);
			}
			drop(rstv_lock);
			let handle = Box::from_raw(phandle);
			// decrement reference count on lib handle
			let result = handle.unload();
			match result {
				Ok(()) => Ok(()),
				Err(_) => Err(()),
			}
		} else {
			Err(())
		}
	}
}