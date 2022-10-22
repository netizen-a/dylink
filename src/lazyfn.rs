use std::{
	cell, ffi,
	mem,
	os::raw::c_char,
	ptr, sync,
};

use windows_sys::Win32::Foundation::PROC;

use crate::loader::*;
use crate::error::*;

pub enum LinkType {
	OpenGL,
	Vulkan,
	General { library: &'static str },
}

pub trait SizeTest<T, U> {
	const SIZE_TEST: () = assert!(mem::size_of::<T>() == mem::size_of::<U>());
}
impl<F> SizeTest<fn(), F> for LazyFn<F> {}

// This can be used safely without the dylink macro.
// `F` can be anything as long as it's the size of a function pointer
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

	/// Can be used to preload functions before called.
	/// If successful, stores address and returns it.
	pub fn link_addr(&self, info: LinkType) -> Result<F, DylinkError> {
		let name = self.name;
		let mut result = Err(DylinkError::new(name, ErrCause::AlreadyLinked));
		self.once.call_once(|| unsafe {
			let loaded_addr = match info {
				LinkType::Vulkan => vkloader(name),
				LinkType::OpenGL => glloader(name),
				LinkType::General { library } => loader(library, name),
			};
			match loaded_addr {
				Some(addr) => {
					// `SizeTest` asserts `F` to be same size an `fn` pointer, so transmute_copy is safe.
					let _ = Self::SIZE_TEST;
					*cell::UnsafeCell::raw_get(&self.addr) = mem::transmute_copy(&addr);
					result = Ok(mem::transmute_copy(&addr));
				}
				None => result = Err(DylinkError::new(name, ErrCause::FnNotFound)),
			}
		});
		result
	}
}
impl<F> std::ops::Deref for LazyFn<F> {
	type Target = F;

	// `addr` is never uninitialized, so `unwrap_unchecked` is safe.
	fn deref(&self) -> &Self::Target { unsafe { self.addr.get().as_ref().unwrap_unchecked() } }
}

/////////////////////////////////////////
// SPECIALIZATION: vkGetDeviceProcAddr //
/////////////////////////////////////////

#[allow(non_upper_case_globals)]
pub(crate) static vkGetDeviceProcAddr: LazyFn<
	extern "system" fn(ptr::NonNull<ffi::c_void>, *const c_char) -> PROC,
> = LazyFn::new("vkGetDeviceProcAddr\0", get_device_proc_addr_init);

// Rust closures can't infer foreign calling conventions, so they must be defined
// seperate from initialization.
extern "system" fn get_device_proc_addr_init(
	device: ptr::NonNull<ffi::c_void>,
	name: *const c_char,
) -> PROC {
	vkGetDeviceProcAddr.once.call_once(|| unsafe {
		let instance = crate::VK_CONTEXT
			.instance
			.load(sync::atomic::Ordering::Acquire);
		let self_name = vkGetDeviceProcAddr.name.as_ptr();
		debug_assert!(
			!instance.is_null() && !name.is_null(),
			"Dylink Error: undefined behavior!"
		);

		*cell::UnsafeCell::raw_get(&vkGetDeviceProcAddr.addr) = mem::transmute(
			crate::example::vkGetInstanceProcAddr(instance as *const _, self_name as *const _)
				.unwrap(),
		);
	});
	vkGetDeviceProcAddr(device, name)
}
