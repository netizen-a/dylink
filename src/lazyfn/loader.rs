// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use std::{
	ffi,
	mem,
	sync::RwLock,
};

use crate::{error::*, vulkan, FnPtr, Result};

pub(crate) unsafe fn vulkan_loader(fn_name: &ffi::CStr) -> Result<FnPtr> {
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

/// `loader` is a generalization for all other dlls.
pub(crate) fn general_loader<L: crate::RTLinker>(
	lib_name: &ffi::CStr,
	fn_name: &ffi::CStr,
) -> Result<FnPtr> {
	use std::collections::HashMap;

	use once_cell::sync::Lazy;

	static DLL_DATA: RwLock<Lazy<HashMap<ffi::CString, crate::LibHandle>>> =
		RwLock::new(Lazy::new(HashMap::default));

	// somehow rust is smart enough to infer that maybe_fn is assigned to only once after branching.
	let maybe_fn;

	let read_lock = DLL_DATA.read().unwrap();
	if let Some(handle) = read_lock.get(lib_name) {
		maybe_fn = L::load_sym(&handle, fn_name);
	} else {
		mem::drop(read_lock);

		let lib_handle = L::load_lib(lib_name);

		if lib_handle.is_invalid() {
			return Err(DylinkError::LibNotLoaded(
				lib_name.to_string_lossy().into_owned(),
			));
		} else {
			maybe_fn = L::load_sym(&lib_handle, fn_name);
			DLL_DATA
				.write()
				.unwrap()
				.insert(lib_name.to_owned(), lib_handle);
		}
	}
	match maybe_fn {
		Some(addr) => Ok(addr),
		None => Err(DylinkError::FnNotFound(
			fn_name.to_str().unwrap().to_owned(),
		)),
	}
}
