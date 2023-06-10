// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
use super::*;

impl <'a> Loader<'a> for System {
	type Data = ffi::c_void;

	fn load_lib(lib_name: &'static ffi::CStr) -> LibHandle<'a, Self::Data> {
		#[cfg(unix)]
		unsafe {
			use crate::os::unix::*;
			dlopen(lib_name.as_ptr(), RTLD_NOW | RTLD_LOCAL)
				.as_ref()
				.into()
		}
		#[cfg(windows)]
		unsafe {
			use crate::os::win32::*;
			let wide_str: Vec<u16> = lib_name
				.to_string_lossy()
				.encode_utf16()
				.chain(std::iter::once(0u16))
				.collect();

			crate::os::win32::LoadLibraryExW(
				wide_str.as_ptr().cast(),
				std::ptr::null_mut(),
				LOAD_LIBRARY_SEARCH_DEFAULT_DIRS | LOAD_LIBRARY_SAFE_CURRENT_DIRS,
			)
			.as_ref()
			.into()
		}
	}

	fn load_sym(
		lib_handle: &LibHandle<'a, Self::Data>,
		fn_name: &'static ffi::CStr,
	) -> crate::FnAddr {
		unsafe {
			crate::os::dlsym(
				lib_handle
					.as_ref()
					.map(|r| r as *const _ as *mut ffi::c_void)
					.unwrap_or(std::ptr::null_mut()),
				fn_name.as_ptr().cast(),
			)
		}
	}
}

impl System {
	#[cfg(feature = "unload")]
	pub unsafe fn unload<const N: usize>(library: &mut lazylib::LazyLib<Self>) -> std::io::Result<()> {
    	use std::{sync::atomic::Ordering, io::Error};
        if let Some(handle) = library.hlib.take() {
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
			Err(Error::new(std::io::ErrorKind::Other, "Dylink Error: library not initialized"))
		}
	}
}
