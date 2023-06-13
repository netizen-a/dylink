// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
use super::*;

#[doc(hidden)]
pub struct SystemHandle(*mut std::ffi::c_void);
unsafe impl Send for SystemHandle {}
impl crate::loader::LibHandle for SystemHandle {
	fn is_invalid(&self) -> bool {
		self.0.is_null()
	}
}

impl Loader<'_> for System {
	type Handle = SystemHandle;

	fn load_lib(lib_name: &'static ffi::CStr) -> Self::Handle {
		#[cfg(unix)]
		unsafe {
			use crate::os::unix::*;
			SystemHandle(dlopen(lib_name.as_ptr(), RTLD_NOW | RTLD_LOCAL))
		}
		#[cfg(windows)]
		unsafe {
			use crate::os::win32::*;
			let wide_str: Vec<u16> = lib_name
				.to_string_lossy()
				.encode_utf16()
				.chain(std::iter::once(0u16))
				.collect();

			SystemHandle(crate::os::win32::LoadLibraryExW(
				wide_str.as_ptr().cast(),
				std::ptr::null_mut(),
				LOAD_LIBRARY_SEARCH_DEFAULT_DIRS | LOAD_LIBRARY_SAFE_CURRENT_DIRS,
			))
		}
	}

	fn load_sym(lib_handle: &Self::Handle, fn_name: &'static ffi::CStr) -> crate::FnAddr {
		unsafe { crate::os::dlsym(lib_handle.0, fn_name.as_ptr().cast()) }
	}
}

impl System {
	#[cfg(feature = "unload")]
	pub unsafe fn unload(library: &lazylib::LazyLib<Self>) -> std::io::Result<()> {
		use std::{io::Error, sync::atomic::Ordering};
		let mut wlock = library.hlib.lock().unwrap();
		if let Some(handle) = wlock.take() {
			let mut rstl_lock = library.rstl.lock().unwrap();
			for (pfn, FnAddrWrapper(init_pfn)) in rstl_lock.drain(..) {
				pfn.store(init_pfn.cast_mut(), Ordering::Release);
			}
			drop(rstl_lock);

			let result = crate::os::dlclose(handle.0);
			if (cfg!(windows) && result == 0) || (cfg!(unix) && result != 0) {
				Err(Error::last_os_error())
			} else {
				Ok(())
			}
		} else {
			Err(Error::new(
				std::io::ErrorKind::Other,
				"Dylink Error: library not initialized",
			))
		}
	}
}
