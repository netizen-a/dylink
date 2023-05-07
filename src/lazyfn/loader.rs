// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use std::{
	ffi::{self, CStr},
	mem,
	sync::RwLock,
};

use std::sync::atomic::{AtomicPtr, Ordering};

use crate::{error::*, vulkan, FnPtr, Result};

use super::os;

pub(crate) unsafe fn vulkan_loader(fn_name: &CStr) -> Result<FnPtr> {
	let mut maybe_fn = match fn_name.to_bytes() {
		b"vkGetInstanceProcAddr" => {
			Some(mem::transmute::<vulkan::PFN_vkGetInstanceProcAddr, FnPtr>(
				vulkan::vkGetInstanceProcAddr,
			))
		}
		b"vkGetDeviceProcAddr" => Some(mem::transmute::<vulkan::PFN_vkGetDeviceProcAddr, FnPtr>(
			vulkan::vkGetDeviceProcAddr,
		)),
		_ => None,
	};
	maybe_fn = match maybe_fn {
		Some(addr) => return Ok(addr),
		None => crate::VK_DEVICE
			.read()
			.expect("failed to get read lock")
			.iter()
			.find_map(|device| vulkan::vkGetDeviceProcAddr(*device, fn_name.as_ptr() as *const _)),
	};
	maybe_fn = match maybe_fn {
		Some(addr) => return Ok(addr),
		None => crate::VK_INSTANCE
			.read()
			.expect("failed to get read lock")
			.iter()
			.find_map(|instance| {
				vulkan::vkGetInstanceProcAddr(*instance, fn_name.as_ptr() as *const ffi::c_char)
			}),
	};
	match maybe_fn {
		Some(addr) => Ok(addr),
		None => vulkan::vkGetInstanceProcAddr(
			vulkan::VkInstance(std::ptr::null()),
			fn_name.as_ptr() as *const ffi::c_char,
		)
		.ok_or(DylinkError::FnNotFound(
			fn_name.to_str().unwrap().to_owned(),
		)),
	}
}

struct LibHandle(AtomicPtr<ffi::c_void>);

impl LibHandle {
	fn is_invalid(&self) -> bool {
		self.0.load(Ordering::Acquire).is_null()
	}
}

impl Clone for LibHandle {
	fn clone(&self) -> Self {
		Self(AtomicPtr::new(self.0.load(Ordering::Acquire)))
	}
}

/// `loader` is a generalization for all other dlls.
pub(crate) fn system_loader(lib_path: &str, fn_name: &CStr) -> Result<FnPtr> {
	use std::collections::HashMap;

	use once_cell::sync::Lazy;

	static DLL_DATA: RwLock<Lazy<HashMap<String, LibHandle>>> =
		RwLock::new(Lazy::new(HashMap::default));

	let read_lock = DLL_DATA.read().unwrap();

	let handle: LibHandle = if let Some(handle) = read_lock.get(lib_path) {
		handle.clone()
	} else {
		mem::drop(read_lock);

		let lib_handle = unsafe {
			#[cfg(windows)]
			{
				let wide_str: Vec<u16> = lib_path
					.encode_utf16()
					.chain(std::iter::once(0u16))
					.collect();
				// miri hates this function, but it works fine.
				LibHandle(AtomicPtr::new(os::win32::LoadLibraryExW(
					wide_str.as_ptr() as *const _,
					std::ptr::null_mut(),
					os::win32::LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
				)))
			}
			#[cfg(unix)]
			{
				let c_str = std::ffi::CString::new(lib_path).unwrap();
				let b_str = c_str.into_bytes_with_nul();
				LibHandle(AtomicPtr::new(os::unix::dlopen(
					b_str.as_ptr().cast(),
					os::unix::RTLD_NOW | os::unix::RTLD_LOCAL,
				)))
			}
		};
		if lib_handle.is_invalid() {
			return Err(DylinkError::LibNotLoaded(
				std::io::Error::last_os_error().to_string(),
			));
		} else {
			DLL_DATA
				.write()
				.unwrap()
				.insert(lib_path.to_owned(), lib_handle.clone());
		}
		lib_handle
	};

	let maybe_fn: Option<FnPtr> = unsafe {
		let raw_handle = handle.0.load(Ordering::Acquire);
		#[cfg(windows)]
		{
			os::win32::GetProcAddress(raw_handle, fn_name.as_ptr().cast())
		}
		#[cfg(unix)]
		{
			let addr: *const ffi::c_void = os::unix::dlsym(raw_handle, fn_name.as_ptr().cast());
			std::mem::transmute(addr)
		}
	};
	match maybe_fn {
		Some(addr) => Ok(addr),
		None => Err(DylinkError::FnNotFound(
			fn_name.to_str().unwrap().to_owned(),
		)),
	}
}
