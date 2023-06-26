// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use super::*;

#[doc(hidden)]
pub struct SysHandle(*mut core::ffi::c_void);
unsafe impl Send for SysHandle {}
impl crate::loader::LibHandle for SysHandle {
	fn is_invalid(&self) -> bool {
		self.0.is_null()
	}
}

impl Loader for SysLoader {
	type Handle = SysHandle;

	fn load_lib(lib_name: &'static ffi::CStr) -> Self::Handle {
		#[cfg(unix)]
		unsafe {
			use crate::os::unix::*;
			SysHandle(
				dlopen(lib_name.as_ptr(), RTLD_NOW | RTLD_LOCAL)
			)
		}
		#[cfg(windows)]
		unsafe {
			use crate::os::win32::*;
			let wide_str: Vec<u16> = lib_name
				.to_string_lossy()
				.encode_utf16()
				.chain(core::iter::once(0u16))
				.collect();

			SysHandle(crate::os::win32::LoadLibraryExW(
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

	fn load_sym(lib_handle: &Self::Handle, fn_name: &'static ffi::CStr) -> crate::FnAddr {
		unsafe { crate::os::dlsym(lib_handle.0, fn_name.as_ptr().cast()) }
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

		let maybe_handle = library
			.hlib
			.swap(core::ptr::null_mut(), Ordering::SeqCst)
			.as_ref();
		let result = if let Some(handle) = maybe_handle {
			let mut rstv_lock = library.rstv.lock().unwrap();
			for (pfn, FnAddrWrapper(init_pfn)) in rstv_lock.drain(..) {
				pfn.store(init_pfn.cast_mut(), Ordering::Release);
			}
			drop(rstv_lock);

			let result = crate::os::dlclose(handle.0);
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
