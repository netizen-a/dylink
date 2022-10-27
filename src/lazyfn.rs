use std::{
	cell, ffi, mem,
	os::raw::c_char,
	ptr,
	sync::{self, atomic},
};

use crate::{error::*, loader::*, FnPtr, Result, VK_CONTEXT};

#[derive(Clone, Copy)]
pub enum LinkType {
	OpenGL,
	Vulkan,
	General { library: &'static str },
}

pub trait AssertSize<T, U> {
	const SIZE_TEST: () = assert!(mem::size_of::<T>() == mem::size_of::<U>());
}
impl<F: Sync + Copy> AssertSize<FnPtr, F> for LazyFn<F> {}

// This can be used safely without the dylink macro.
// `F` can be anything as long as it's the size of a function pointer
pub struct LazyFn<F: Sync + Copy> {
	addr:   cell::UnsafeCell<F>,
	once:   sync::Once,
	status: cell::UnsafeCell<Option<ErrorKind>>,
}

unsafe impl<F: Sync + Copy> Sync for LazyFn<F> {}

impl<F: Sync + Copy> LazyFn<F> {
	#[inline]
	pub const fn new(thunk: F) -> Self {
		Self {
			addr:   cell::UnsafeCell::new(thunk),
			once:   sync::Once::new(),
			status: cell::UnsafeCell::new(None),
		}
	}

	// Can be used to preload functions, but the overhead is too insignificant to matter.

	/// If successful, stores address and returns it.
	pub fn link_lib(&self, fn_name: &str, info: LinkType) -> Result<F> {
		self.once.call_once(|| unsafe {
			let maybe = match info {
				LinkType::Vulkan => vkloader(
					fn_name,
					VK_CONTEXT.instance.load(atomic::Ordering::Acquire),
					VK_CONTEXT.device.load(atomic::Ordering::Acquire),
				),
				LinkType::OpenGL => glloader(fn_name),
				LinkType::General { library } => loader(library, fn_name),
			};
			match maybe {
				Ok(addr) => {
					// `AssertSize` asserts sizeof(F) = sizeof(fn), so `transmute_copy` is safe.
					let _ = Self::SIZE_TEST;
					cell::UnsafeCell::raw_get(&self.addr).write(mem::transmute_copy(&addr));
				}
				Err(DylinkError { kind, .. }) => {
					cell::UnsafeCell::raw_get(&self.status).write(Some(kind));
				}
			}
		});
		// `call_once` is blocking, so `self.status` is read-only
		// by this point. Race conditions shouldn't occur.
		match unsafe { *self.status.get() } {
			None => Ok(**self),
			Some(kind) => Err(DylinkError::new(fn_name.to_owned(), kind)),
		}
	}
}
impl<F: Sync + Copy> std::ops::Deref for LazyFn<F> {
	type Target = F;

	// `addr` is never uninitialized, so `unwrap_unchecked` is safe.
	fn deref(&self) -> &Self::Target { unsafe { self.addr.get().as_ref().unwrap_unchecked() } }
}

/////////////////////////////////////////
// SPECIALIZATION: vkGetDeviceProcAddr //
/////////////////////////////////////////

#[allow(non_upper_case_globals)]
pub(crate) static vkGetDeviceProcAddr: LazyFn<
	unsafe extern "system" fn(*const ffi::c_void, *const c_char) -> Option<FnPtr>,
> = LazyFn::new(get_device_proc_addr_init);

#[inline(never)]
unsafe extern "system" fn get_device_proc_addr_init(
	device: *const ffi::c_void,
	name: *const c_char,
) -> Option<FnPtr> {
	vkGetDeviceProcAddr.once.call_once(|| {
		let fn_ptr = crate::loader::vkloader(
			"vkGetDeviceProcAddr",
			VK_CONTEXT.instance.load(atomic::Ordering::Acquire),
			ptr::null(),
		)
		.unwrap();
		*cell::UnsafeCell::raw_get(&vkGetDeviceProcAddr.addr) = mem::transmute(fn_ptr);
	});
	vkGetDeviceProcAddr(device, name)
}
