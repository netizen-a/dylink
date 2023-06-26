// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use super::*;

impl Loader for SysLoader {
	fn is_invalid(&self) -> bool {
		self.0.is_null()
	}
	//type Handle = SysHandle;
	/// Increments reference count to handle, and returns handle if successful.
	fn load_lib(lib_name: &'static ffi::CStr) -> Self {
		#[cfg(unix)]
		unsafe {
			use crate::os::unix::*;
			Self(dlopen(lib_name.as_ptr(), RTLD_NOW | RTLD_LOCAL))
		}
		#[cfg(windows)]
		unsafe {
			use crate::os::win32::*;
			let wide_str: Vec<u16> = lib_name
				.to_string_lossy()
				.encode_utf16()
				.chain(core::iter::once(0u16))
				.collect();

			Self(crate::os::win32::LoadLibraryExW(
				wide_str.as_ptr().cast(),
				core::ptr::null_mut(),
				LOAD_LIBRARY_SEARCH_DEFAULT_DIRS | LOAD_LIBRARY_SAFE_CURRENT_DIRS,
			))
		}
		#[cfg(wasm)]
		{
			todo!()
		}
	}

	fn load_sym(&self, fn_name: &'static ffi::CStr) -> crate::FnAddr {
		unsafe { crate::os::dlsym(self.0, fn_name.as_ptr().cast()) }
	}
}

impl SysLoader {
	/// Unloads the library and resets all associated function pointers to uninitialized state.
	///
	/// # Errors
	/// This may error if library is uninitialized.
	#[cfg(any(feature = "unload", doc))]
	pub unsafe fn unload<const N: usize>(library: &lazylib::LazyLib<Self, N>) -> Result<(), ()> {
		use core::sync::atomic::Ordering;
		// lock
		while library.atml.swap(true, Ordering::Acquire) {
			core::hint::spin_loop()
		}

		let phandle = library.hlib.swap(core::ptr::null_mut(), Ordering::SeqCst);
		let result = if !phandle.is_null() {
			let mut rstv_lock = library.rstv.lock().unwrap();
			for (pfn, FnAddrWrapper(init_pfn)) in rstv_lock.drain(..) {
				pfn.store(init_pfn.cast_mut(), Ordering::Release);
			}
			drop(rstv_lock);
			let handle = Box::from_raw(phandle);
			// decrement reference count on lib handle
			let result = crate::os::dlclose(handle.0);
			drop(handle);

			if (cfg!(windows) && result == 0) || (cfg!(unix) && result != 0) {
				Err(())
			} else {
				Ok(())
			}
		} else {
			Err(())
		};
		// unlock
		library.atml.store(false, Ordering::Release);
		result
	}
}
