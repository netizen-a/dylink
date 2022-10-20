use std::{cell, ffi, mem, os::raw::c_char, sync};

use windows_sys::Win32::Foundation::PROC;

use crate::loader::*;

pub enum LinkType {
	OpenGL,
	Vulkan,
	General { library: &'static str },
}

pub struct LazyFn<F> {
	addr: cell::UnsafeCell<F>,
	once: sync::Once,
	name: &'static str,
}

// both traits are enforced by the use of `Once`, so it should be safe to add.
unsafe impl<F> Sync for LazyFn<F> {}
unsafe impl<F> Send for LazyFn<F> {}

impl<F> LazyFn<F> {
	#[inline]
	pub const fn new(name: &'static str, thunk: F) -> Self {
		Self {
			addr: cell::UnsafeCell::new(thunk),
			once: sync::Once::new(),
			name,
		}
	}

	// This function can be used to load proactively
	pub fn link_addr(&self, info: LinkType) -> Result<(), String> {
		let mut result = Err(format!(
			"Dylink Error: function `{}` already linked",
			self.name
		));
		self.once.call_once(|| unsafe {
			let loader = match info {
				LinkType::Vulkan => vkloader(self.name),
				LinkType::OpenGL => glloader(self.name),
				LinkType::General { library } => loader(library, self.name),
			};
			match loader {
				Some(loader) => {
					*cell::UnsafeCell::raw_get(&self.addr) = mem::transmute_copy(&loader);
					result = Ok(());
				}
				None => result = Err(format!("Dylink Error: function `{}` not found", self.name)),
			}
		});
		result
	}
}
impl<F> std::ops::Deref for LazyFn<F> {
	type Target = F;

	#[inline]
	fn deref(&self) -> &Self::Target { unsafe { self.addr.get().as_ref().unwrap_unchecked() } }
}

/////////////////////////////////////////
// SPECIALIZATION: vkGetDeviceProcAddr //
/////////////////////////////////////////

extern "system" fn get_device_proc_addr_init(
	device: *const ffi::c_void,
	name: *const c_char,
) -> PROC {
	vkGetDeviceProcAddr.once.call_once(|| unsafe {
		*cell::UnsafeCell::raw_get(&vkGetDeviceProcAddr.addr) = mem::transmute(
			crate::example::vkGetInstanceProcAddr(
				crate::VK_CONTEXT
					.instance
					.load(sync::atomic::Ordering::Acquire) as *const _,
				vkGetDeviceProcAddr.name.as_ptr() as *const _,
			)
			.unwrap(),
		);
	});
	vkGetDeviceProcAddr(device, name)
}

#[allow(non_upper_case_globals)]
pub(crate) static vkGetDeviceProcAddr: LazyFn<
	extern "system" fn(*const ffi::c_void, *const c_char) -> PROC,
> = LazyFn::new("vkGetDeviceProcAddr\0", get_device_proc_addr_init);
