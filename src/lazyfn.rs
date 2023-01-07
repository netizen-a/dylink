use std::{cell, mem, sync};

use crate::*;

mod loader;

#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash, Debug)]
pub enum LinkType {
	OpenGL,
	Vulkan,
	Normal(&'static [&'static str]),
}

/*impl<const N: usize> LinkType<N> {
	const fn lib_count(&self) -> usize {
		N
	}
}*/

// This can be used safely without the dylink macro.
// `F` can be anything as long as it's the size of a function pointer
pub struct LazyFn<F: 'static> {
	// it's imperative that LazyFn manages once, so that `LazyFn::load` is sound.
	once: sync::Once,
	// this is here to track the state of the instance.
	status: cell::UnsafeCell<Option<ErrorKind>>,
	// The function to be called.
	// Non-function types can be stored, but obviously can't be called (call ops aren't overloaded).
	addr: cell::UnsafeCell<F>,
}

impl<F: 'static> LazyFn<F> {
	/// Initializes a `LazyFn` object with all the necessary information for `LazyFn::link` to work.
	/// # Panic
	/// Type `F` must be the same size as `FnPtr`.
	#[inline]
	pub const fn new(thunk: F) -> Self {
		assert!(mem::size_of::<FnPtr>() == mem::size_of::<F>());
		Self {
			addr: cell::UnsafeCell::new(thunk),
			once: sync::Once::new(),
			status: cell::UnsafeCell::new(None),
		}
	}

	/// If successful, stores address and returns it.
	pub fn load(&self, fn_name: &'static ffi::CStr, link_ty: LinkType) -> Result<&F> {
		let str_name = fn_name.to_str().unwrap();
		self.once.call_once(|| unsafe {
			let maybe = match link_ty {
				LinkType::Vulkan => {
					let device_read_lock = VK_DEVICE.read().expect("failed to get read lock");
					match device_read_lock.iter().find_map(|device| {
						loader::vkGetDeviceProcAddr(*device, fn_name.as_ptr() as *const _)
					}) {
						Some(addr) => Ok(addr),
						None => {
							mem::drop(device_read_lock);
							let instance_read_lock =
								VK_INSTANCE.read().expect("failed to get read lock");
							// check other instances if fails in case one has a higher available version number
							match instance_read_lock.iter().find_map(|instance| {
								loader::vkGetInstanceProcAddr(*instance, fn_name.as_ptr())
							}) {
								Some(addr) => Ok(addr),
								None => loader::vkGetInstanceProcAddr(
									ffi::VkInstance(std::ptr::null()),
									fn_name.as_ptr(),
								)
								.ok_or(error::DylinkError::new(Some(str_name), ErrorKind::FnNotFound)),
							}
						}
					}
				}
				LinkType::OpenGL => loader::glloader(str_name),
				LinkType::Normal(lib_list) => {
					let mut result = Err(error::DylinkError::new(None, ErrorKind::ListNotFound));
					for lib_name in lib_list {
						match loader::loader(ffi::OsStr::new(lib_name), str_name) {
							Ok(addr) => {
								result = Ok(addr);
								// success! lib and function retrieved!
								break;
							}
							Err(err) => {
								if let ErrorKind::FnNotFound = err.kind() {
									result = Err(err);
									// lib detected, but function failed to load
									break;
								}
							}
						}
					}
					result
				}
			};
			match maybe {
				Ok(addr) => {
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
			Some(kind) => Err(DylinkError::new(Some(str_name), kind)),
		}
	}
}

unsafe impl<F: 'static> Send for LazyFn<F> {}
unsafe impl<F: 'static> Sync for LazyFn<F> {}

impl<F: 'static> std::ops::Deref for LazyFn<F> {
	type Target = F;

	fn deref(&self) -> &Self::Target {
		self.as_ref()
	}
}

impl<F: 'static> std::convert::AsRef<F> for LazyFn<F> {
	// `addr` is never uninitialized, so `unwrap_unchecked` is safe.
	#[inline]
	fn as_ref(&self) -> &F {
		unsafe { self.addr.get().as_ref().unwrap_unchecked() }
	}
}
