// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use std::{ffi::{self, OsStr}, mem, sync::RwLock, path::{Path, PathBuf}};

use crate::{error::*, vulkan, FnPtr, Result};

pub(crate) unsafe fn vulkan_loader(fn_name: &str) -> Result<FnPtr> {
	let mut maybe_fn = match fn_name {
		"vkGetInstanceProcAddr" => Some(mem::transmute::<
			vulkan::PFN_vkGetInstanceProcAddr,
			FnPtr,
		>(vulkan::vkGetInstanceProcAddr)),
		"vkGetDeviceProcAddr" => Some(mem::transmute::<
			vulkan::PFN_vkGetDeviceProcAddr,
			FnPtr,
		>(vulkan::vkGetDeviceProcAddr)),
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
		None => crate::VK_INSTANCE
			.read()
			.expect("failed to get read lock")
			.iter()
			.find_map(|instance| {
				vulkan::vkGetInstanceProcAddr(*instance, fn_name.as_ptr() as *const ffi::c_char)
			})
	};
	match maybe_fn {
		Some(addr) => Ok(addr),
		None => vulkan::vkGetInstanceProcAddr(
			vulkan::VkInstance(std::ptr::null()),
			fn_name.as_ptr() as *const ffi::c_char,
		)
		.ok_or(DylinkError::new(
			Some(fn_name.to_owned()),
			ErrorKind::FnNotFound,
		)),
	}
}

/// `loader` is a generalization for all other dlls.
pub(crate) fn system_loader(lib_path: &Path, fn_name: &OsStr) -> Result<FnPtr> {
	use std::collections::HashMap;

	use once_cell::sync::Lazy;
	#[cfg(windows)]
	use windows_sys::Win32::System::LibraryLoader::{
		GetProcAddress, LoadLibraryExW, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
	};

	static DLL_DATA: RwLock<Lazy<HashMap<PathBuf, isize>>> =
		RwLock::new(Lazy::new(HashMap::default));

	let path_str = lib_path.to_str().unwrap();
	
	let fn_str = fn_name.to_str().unwrap();

	let read_lock = DLL_DATA.read().unwrap();

	let handle: isize = if let Some(handle) = read_lock.get(lib_path) {
		*handle
	} else {
		mem::drop(read_lock);

		let lib_handle = unsafe {
			#[cfg(windows)]
			{
				use std::os::windows::ffi::OsStrExt;
				let os_str = lib_path.as_os_str();
				let mut wide_str: Vec<u16> = os_str.encode_wide().collect();
				wide_str.push(0);
				// miri hates this function, but it works fine.
				LoadLibraryExW(
					wide_str.as_ptr() as *const _,
					0,
					LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
				)
			}
			#[cfg(unix)]
			{				
				let c_str = std::ffi::CString::new(path_str).unwrap();
				let filename = c_str.into_bytes_with_nul();
				libc::dlopen(
					filename.as_ptr() as *const _,
					libc::RTLD_NOW | libc::RTLD_LOCAL,
				) as isize
			}
		};
		if lib_handle == 0 {
			return Err(DylinkError::new(Some(path_str.to_owned()), ErrorKind::LibNotFound));
		} else {
			DLL_DATA.write().unwrap().insert(lib_path.to_owned(), lib_handle);
		}
		lib_handle
	};

	let maybe_fn: Option<FnPtr> = unsafe {
		#[cfg(windows)]
		{
			GetProcAddress(handle, fn_str.as_ptr() as *const _)
		}
		#[cfg(unix)]
		{
			let addr: *const libc::c_void =
				libc::dlsym(handle as *mut libc::c_void, fn_str.as_ptr() as *const _);
			std::mem::transmute(addr)
		}
	};
	match maybe_fn {
		Some(addr) => Ok(addr),
		None => Err(DylinkError::new(Some(path_str.to_owned()), ErrorKind::FnNotFound)),
	}
}
