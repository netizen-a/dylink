// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use crate::loader;
use crate::loader::LibHandle;
use crate::loader::Loader;
use crate::FnAddr;
use core::ffi::CStr;
use core::sync::atomic::AtomicPtr;
use core::sync::atomic::Ordering;
use alloc::boxed::Box;


#[cfg(feature="unload")]
use std::sync::Mutex;

// this wrapper struct is the bane of my existance...
#[derive(Debug)]
pub(crate) struct FnAddrWrapper(pub FnAddr);
unsafe impl Send for FnAddrWrapper {}

#[derive(Debug)]
pub struct LazyLib<L: Loader = loader::SysLoader, const N: usize = 1> {
	libs: [&'static CStr; N],
	// library handle
	pub(crate) hlib: AtomicPtr<L::Handle>,
	// reset vector
	#[cfg(feature = "unload")]
	pub(crate) rstv: Mutex<Vec<(&'static AtomicPtr<()>, FnAddrWrapper)>>,
}

impl<L: Loader, const N: usize> LazyLib<L, N> {
	pub const fn new(libs: [&'static CStr; N]) -> Self {
		Self {
			libs,
			hlib: AtomicPtr::new(core::ptr::null_mut()),
			#[cfg(feature = "unload")]
			rstv: Mutex::new(Vec::new()),
		}
	}
	/// loads function from library synchronously and binds library handle internally to dylink.
	///
	/// If the library is already bound, the bound handle will be used for loading the function.
	pub unsafe fn find_sym(
		&self,
		sym: &'static CStr,
		_init: FnAddr,
		_atom: &'static AtomicPtr<()>,
	) -> crate::FnAddr {
		if let None = self.hlib.load(Ordering::Acquire).as_ref() {
			for lib_name in self.libs {
				let handle = L::load_lib(lib_name);
				if !handle.is_invalid() {
					self.hlib.store(Box::into_raw(Box::new(handle)), Ordering::Release);
				}
			}
		}
		if let Some(lib_handle) = self.hlib.load(Ordering::Acquire).as_ref() {
			#[cfg(feature = "unload")]
			self.rstv
				.lock()
				.unwrap()
				.push((_atom, FnAddrWrapper(_init)));
			L::load_sym(&lib_handle, sym)
		} else {
			core::ptr::null()
		}
	}
}

impl <L: Loader, const N: usize> Drop for LazyLib<L, N> {
	fn drop(&mut self) {
		let maybe_handle = self.hlib.load(Ordering::Relaxed);
		if !maybe_handle.is_null() {
			unsafe {				
				drop(Box::from_raw(maybe_handle));
			}
		}
	}
}