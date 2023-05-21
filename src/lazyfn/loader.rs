// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use std::{ffi, mem, sync::RwLock, marker::PhantomData};

use crate::{error::*, vulkan, DylinkResult, FnPtr};

pub(crate) unsafe fn vulkan_loader(fn_name: &ffi::CStr) -> Option<FnPtr> {
	let mut maybe_fn = crate::VK_DEVICE
		.read()
		.expect("failed to get read lock")
		.iter()
		.find_map(|device| {
			vulkan::vkGetDeviceProcAddr(*device, fn_name.as_ptr() as *const ffi::c_char)
		});
	maybe_fn = match maybe_fn {
		Some(addr) => return Some(addr),
		None => crate::VK_INSTANCE
			.read()
			.expect("failed to get read lock")
			.iter()
			.find_map(|instance| {
				vulkan::vkGetInstanceProcAddr(*instance, fn_name.as_ptr() as *const ffi::c_char)
			}),
	};
	match maybe_fn {
		Some(addr) => Some(addr),
		None => vulkan::vkGetInstanceProcAddr(
			vulkan::VkInstance::null(),
			fn_name.as_ptr() as *const ffi::c_char,
		),
	}
}

/// `loader` is a generalization for all other dlls.
pub(crate) fn general_loader<L: crate::RTLinker>(
	lib_name: &ffi::CStr,
	fn_name: &ffi::CStr,
) -> DylinkResult<FnPtr>
where
	L::Data: Send + Sync,
{
	static DLL_DATA: RwLock<Vec<(ffi::CString, crate::LibHandle<ffi::c_void>)>> = RwLock::new(Vec::new());

	// somehow rust is smart enough to infer that maybe_fn is assigned to only once after branching.
	let maybe_fn;

	let read_lock = DLL_DATA.read().unwrap();
	match read_lock.binary_search_by_key(&lib_name, |(k, _)| k) {
		Ok(index) => {
			let handle = crate::LibHandle::<L::Data>(read_lock[index].1 .0.cast(), PhantomData);
			maybe_fn = L::load_sym(&handle, fn_name)
		}
		Err(index) => {
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
					.insert(index, (lib_name.to_owned(), lib_handle.as_opaque()));
			}
		}
	}
	match maybe_fn {
		Some(addr) => Ok(addr),
		None => Err(DylinkError::FnNotFound(
			fn_name.to_str().unwrap().to_owned(),
		)),
	}
}
