use std::{
	cell, mem,
	sync::{self, atomic},
};

use crate::{error::*, loader::*, FnPtr, Result, VK_CONTEXT};


#[derive(Clone, Copy)]
pub enum LinkType {
	OpenGL,
	Vulkan,
	Normal(&'static str),
}

pub trait AssertSize<T, U> {
	const ASSERT_SIZE: () = assert!(mem::size_of::<T>() == mem::size_of::<U>());
}
impl<F: 'static> AssertSize<FnPtr, F> for LazyFn<F> {}

// This can be used safely without the dylink macro.
// `F` can be anything as long as it's the size of a function pointer
pub struct LazyFn<F: 'static> {
	name:   &'static str,
	addr:   cell::UnsafeCell<F>,
	link_ty: LinkType,
	once:   sync::Once,
	status: cell::UnsafeCell<Option<ErrorKind>>,
}

unsafe impl<F: 'static> Sync for LazyFn<F> {}

impl<F: 'static> LazyFn<F> {
	#[inline]
	pub const fn new(name: &'static str, thunk: F, link_ty: LinkType) -> Self {
		Self {
			name,
			addr: cell::UnsafeCell::new(thunk),
			link_ty,
			once: sync::Once::new(),
			status: cell::UnsafeCell::new(None),
		}
	}

	/// If successful, stores address and returns it.
	pub fn link(&self) -> Result<&F> {
		let fn_name = self.name;
		self.once.call_once(|| unsafe {
			let maybe = match self.link_ty {
				LinkType::Vulkan => vkloader(
					fn_name,
					VK_CONTEXT.instance.load(atomic::Ordering::Acquire),
				),
				LinkType::OpenGL => glloader(fn_name),
				LinkType::Normal(library) => loader(library, fn_name),
			};
			match maybe {
				Ok(addr) => {
					// `AssertSize` asserts sizeof(F) = sizeof(fn), so `transmute_copy` is safe.
					#[allow(clippy::let_unit_value)]
					let _ = Self::ASSERT_SIZE;
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
			None => Ok(self.as_ref()),
			Some(kind) => Err(DylinkError::new(fn_name, kind)),
		}
	}
}
impl<F: 'static> std::ops::Deref for LazyFn<F> {
	type Target = F;

	fn deref(&self) -> &Self::Target { self.as_ref() }
}

impl<F: 'static> std::convert::AsRef<F> for LazyFn<F> {
	// `addr` is never uninitialized, so `unwrap_unchecked` is safe.
	fn as_ref(&self) -> &F { unsafe { self.addr.get().as_ref().unwrap_unchecked() } }
}


// vkGetDeviceProcAddr must be implemented manually to avoid recursion
// 
// #[allow(non_upper_case_globals)]
// pub(crate) static vkGetDeviceProcAddr: LazyFn<
// 	unsafe extern "system" fn(*const std::ffi::c_void, *const ffi::c_char) -> Option<FnPtr>,
// > = LazyFn::new("vkGetDeviceProcAddr", get_device_proc_addr_init);
// 
// #[inline(never)]
// unsafe extern "system" fn get_device_proc_addr_init(
// 	device: *const std::ffi::c_void,
// 	name: *const ffi::c_char,
// ) -> Option<FnPtr> {
// 	vkGetDeviceProcAddr.once.call_once(|| {
// 		let fn_ptr = crate::loader::vkloader(
// 			vkGetDeviceProcAddr.name,
// 			VK_CONTEXT.instance.load(atomic::Ordering::Acquire),
// 			ptr::null(),
// 		)
// 		.unwrap();
// 		*cell::UnsafeCell::raw_get(&vkGetDeviceProcAddr.addr) = mem::transmute(fn_ptr);
// 	});
// 	vkGetDeviceProcAddr(device, name)
// }



