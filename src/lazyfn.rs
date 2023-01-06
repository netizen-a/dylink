use std::{cell, mem, sync};

use crate::*;

mod loader;

#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash, Debug)]
pub enum LinkType<const N: usize> {
	OpenGL,
	Vulkan,
	Normal([&'static str; N]),
}

impl<const N: usize> LinkType<N> {
	const fn lib_count(&self) -> usize {
		N
	}
}

// This can be used safely without the dylink macro.
// `F` can be anything as long as it's the size of a function pointer
pub struct LazyFn<F: 'static, const N: usize = 1> {
	// used to retrieve function address
	name: &'static ffi::CStr,
	// The function to be called.
	// Non-function types can be stored, but obviously can't be called (call ops aren't overloaded).
	addr: cell::UnsafeCell<F>,
	link_ty: LinkType<N>,
	once: sync::Once,
	status: cell::UnsafeCell<Option<ErrorKind>>,
}

impl<F: 'static, const N: usize> LazyFn<F, N> {
	/// Initializes a `LazyFn` object with all the necessary information for `LazyFn::link` to work.
	/// # Panic
	/// Type `F` must be the same size as `FnPtr`.
	#[inline]
	pub const fn new(name: &'static ffi::CStr, thunk: F, link_ty: LinkType<N>) -> Self {
		assert!(mem::size_of::<FnPtr>() == mem::size_of::<F>());
		assert!(link_ty.lib_count() != 0);
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
		let name = self.name.to_str().unwrap();
		self.once.call_once(|| unsafe {
			let maybe = match self.link_ty {
				LinkType::Vulkan => {
					let device_read_lock = VK_DEVICE.read().expect("failed to get read lock");
					match device_read_lock.iter().find_map(|device| {
						loader::vkGetDeviceProcAddr(*device, self.name.as_ptr() as *const _)
					}) {
						Some(addr) => Ok(addr),
						None => {
							mem::drop(device_read_lock);
							let instance_read_lock =
								VK_INSTANCE.read().expect("failed to get read lock");
							// check other instances if fails in case one has a higher available version number
							match instance_read_lock.iter().find_map(|instance| {
								loader::vkGetInstanceProcAddr(*instance, self.name.as_ptr())
							}) {
								Some(addr) => Ok(addr),
								None => loader::vkGetInstanceProcAddr(
									ffi::VkInstance(std::ptr::null()),
									self.name.as_ptr(),
								)
								.ok_or(error::DylinkError::new(Some(name), ErrorKind::FnNotFound)),
							}
						}
					}
				}
				LinkType::OpenGL => loader::glloader(name),
				LinkType::Normal(lib_list) => {
					let mut result = Err(error::DylinkError::new(None, ErrorKind::ListNotFound));
					for lib_name in lib_list {
						match loader::loader(ffi::OsStr::new(lib_name), name) {
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
			Some(kind) => Err(DylinkError::new(Some(name), kind)),
		}
	}
}

unsafe impl<F: 'static, const N: usize> Send for LazyFn<F, N> {}
unsafe impl<F: 'static, const N: usize> Sync for LazyFn<F, N> {}

impl<F: 'static, const N: usize> std::ops::Deref for LazyFn<F, N> {
	type Target = F;

	fn deref(&self) -> &Self::Target {
		self.as_ref()
	}
}

impl<F: 'static, const N: usize> std::convert::AsRef<F> for LazyFn<F, N> {
	// `addr` is never uninitialized, so `unwrap_unchecked` is safe.
	#[inline]
	fn as_ref(&self) -> &F {
		unsafe { self.addr.get().as_ref().unwrap_unchecked() }
	}
}
