// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use std::{ffi, mem, sync::RwLock};

use crate::{error::*, vulkan, FnPtr, Result};

pub unsafe fn vulkan_loader(fn_name: &'static str) -> Result<FnPtr> {
	let mut maybe_fn = match fn_name {
		"vkGetInstanceProcAddr" => Some(mem::transmute::<
			unsafe extern "system" fn(
				vulkan::VkInstance,
				*const ffi::c_char,
			) -> Option<FnPtr>,
			FnPtr,
		>(vulkan::vkGetInstanceProcAddr)),
		"vkGetDeviceProcAddr" => Some(mem::transmute::<
			unsafe extern "system" fn(
				vulkan::VkDevice,
				*const ffi::c_char,
			) -> Option<FnPtr>,
			FnPtr,
		>(*vulkan::vkGetDeviceProcAddr.as_ref())),
		_ => None,
	};	
	maybe_fn = match maybe_fn {
		Some(addr) => return Ok(addr),
		None => crate::VK_DEVICE
			.read()
			.expect("failed to get read lock")
			.iter()
			.find_map(|device| {
				vulkan::vkGetDeviceProcAddr(*device, fn_name.as_ptr() as *const _)
			}),
	};
	maybe_fn = match maybe_fn {
		Some(addr) => return Ok(addr),
		None => {
			let instance_read_lock =
				crate::VK_INSTANCE.read().expect("failed to get read lock");
			// check other instances if fails in case one has a higher available version number
			instance_read_lock.iter().find_map(|instance| {
				vulkan::vkGetInstanceProcAddr(*instance, fn_name.as_ptr() as *const ffi::c_char)
			})
		}
	};
	match maybe_fn {
		Some(addr) => Ok(addr),
		None => vulkan::vkGetInstanceProcAddr(
			vulkan::VkInstance(std::ptr::null()),
			fn_name.as_ptr() as *const ffi::c_char,
		)
		.ok_or(DylinkError::new(
			Some(fn_name),
			ErrorKind::FnNotFound,
		)),
	}
}

/// `loader` is a generalization for all other dlls.
pub fn system_loader(lib_name: &'static ffi::OsStr, fn_name: &'static str) -> Result<FnPtr> {
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
				// miri hates this function, but it works fine.
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
