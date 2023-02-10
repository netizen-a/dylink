// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use std::{mem, sync::RwLock};

use crate::{error::*, ffi, lazyfn, FnPtr, Result};
// dylink_macro internally uses dylink as it's root namespace,
// but since we are in dylink the namespace is actually named `self`.
// this is just here to resolve the missing namespace issue.
extern crate self as dylink;

// windows and linux are fully tested and useable as of this comment.
// macos should theoretically work, but it's untested.
// This function is in itself an axiom of the vulkan specialization.
#[cfg_attr(windows, crate::dylink(name = "vulkan-1.dll"))]
#[cfg_attr(
	all(unix, not(target_os = "macos")),
	crate::dylink(any(name = "libvulkan.so.1", name = "libvulkan.so"))
)]
#[cfg_attr(
	target_os = "macos",
	crate::dylink(any(
		name = "libvulkan.dylib",
		name = "libvulkan.1.dylib",
		name = "libMoltenVK.dylib"
	))
)]
extern "system" {
	pub(super) fn vkGetInstanceProcAddr(
		instance: ffi::VkInstance,
		pName: *const ffi::c_char,
	) -> Option<FnPtr>;
}

// vkGetDeviceProcAddr must be implemented manually to avoid recursion
#[allow(non_snake_case)]
pub(super) unsafe extern "system" fn vkGetDeviceProcAddr(
	device: ffi::VkDevice,
	name: *const ffi::c_char,
) -> Option<FnPtr> {
	static DYN_FUNC: lazyfn::LazyFn<
		unsafe extern "system" fn(ffi::VkDevice, *const ffi::c_char) -> Option<FnPtr>,
	> = lazyfn::LazyFn::new(initial_fn);

	unsafe extern "system" fn initial_fn(
		device: ffi::VkDevice,
		name: *const ffi::c_char,
	) -> Option<FnPtr> {
		DYN_FUNC.once.call_once(|| {
			let read_lock = crate::VK_INSTANCE.read().expect("failed to get read lock");
			const FN_NAME: &'static ffi::CStr =
				unsafe { ffi::CStr::from_bytes_with_nul_unchecked(b"vkGetDeviceProcAddr\0") };
			// check other instances if fails in case one has a higher available version number
			let fn_ptr = read_lock
				.iter()
				.find_map(|instance| vkGetInstanceProcAddr(*instance, FN_NAME.as_ptr()));
			*std::cell::UnsafeCell::raw_get(&DYN_FUNC.addr) = mem::transmute(fn_ptr);
		});
		DYN_FUNC(device, name)
	}
	DYN_FUNC(device, name)
}

/// `loader` is a generalization for all other dlls.
pub fn loader(lib_name: &'static ffi::OsStr, fn_name: &'static str) -> Result<FnPtr> {
	use std::collections::HashMap;

	use once_cell::sync::Lazy;
	#[cfg(windows)]
	use windows_sys::Win32::System::LibraryLoader::{
		GetProcAddress, LoadLibraryExW, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
	};

	static DLL_DATA: RwLock<Lazy<HashMap<&'static ffi::OsStr, isize>>> =
		RwLock::new(Lazy::new(HashMap::default));

	let read_lock = DLL_DATA.read().unwrap();

	let handle: isize = if let Some(handle) = read_lock.get(lib_name) {
		*handle
	} else {
		mem::drop(read_lock);

		let lib_handle = unsafe {
			#[cfg(windows)]
			{
				use std::os::windows::ffi::OsStrExt;
				let wide_str: Vec<u16> = lib_name.encode_wide().collect();
				LoadLibraryExW(
					wide_str.as_ptr() as *const _,
					0,
					LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
				)
			}
			#[cfg(unix)]
			{
				use std::os::unix::ffi::OsStrExt;
				libc::dlopen(
					lib_name.as_bytes().as_ptr() as *const _,
					libc::RTLD_NOW | libc::RTLD_LOCAL,
				) as isize
			}
		};
		if lib_handle == 0 {
			return Err(DylinkError::new(lib_name.to_str(), ErrorKind::LibNotFound));
		} else {
			DLL_DATA.write().unwrap().insert(lib_name, lib_handle);
		}
		lib_handle
	};

	let maybe_fn: Option<FnPtr> = unsafe {
		#[cfg(windows)]
		{
			GetProcAddress(handle, fn_name.as_ptr() as *const _)
		}
		#[cfg(unix)]
		{
			let addr: *const libc::c_void =
				libc::dlsym(handle as *mut libc::c_void, fn_name.as_ptr() as *const _);
			std::mem::transmute(addr)
		}
	};
	match maybe_fn {
		Some(addr) => Ok(addr),
		None => Err(DylinkError::new(Some(fn_name), ErrorKind::FnNotFound)),
	}
}
