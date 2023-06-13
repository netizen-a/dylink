// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use crate::loader;
use crate::loader::LibHandle;
use crate::loader::Loader;
use crate::FnAddr;
use std::ffi::CStr;
use std::sync::atomic::AtomicPtr;
use std::sync::Mutex;

// this wrapper struct is the bane of my existance...
#[derive(Debug)]
pub(crate) struct FnAddrWrapper(pub FnAddr);
unsafe impl Send for FnAddrWrapper {}

#[derive(Debug)]
pub struct LazyLib<'a, L: Loader<'a> = loader::SystemLoader> {
	libs: &'a [&'static CStr],
	// library handles sorted by name
	pub(crate) hlib: Mutex<Option<L::Handle>>,
	// reset lock vector
	#[cfg(feature = "unload")]
	pub(crate) rstl: Mutex<Vec<(&'static AtomicPtr<()>, FnAddrWrapper)>>,
}

impl<'a, L: Loader<'a>> LazyLib<'a, L> {
	pub const fn new(libs: &'a [&'static CStr]) -> Self {
		Self {
			libs,
			hlib: Mutex::new(None),
			#[cfg(feature = "unload")]
			rstl: Mutex::new(Vec::new()),
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
		let mut lock = self.hlib.lock().unwrap();
		if let None = *lock {
			for lib_name in self.libs {
				let handle = L::load_lib(lib_name);
				if !handle.is_invalid() {
					*lock = Some(handle);
				}
			}
		}
		if let Some(ref lib_handle) = *lock {
			#[cfg(feature = "unload")]
			self.rstl
				.lock()
				.unwrap()
				.push((_atom, FnAddrWrapper(_init)));
			L::load_sym(&lib_handle, sym)
		} else {
			std::ptr::null()
		}
	}
}
