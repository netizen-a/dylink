// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
use core::{
	cell, ffi, mem,
	sync::atomic::{AtomicBool, AtomicPtr, Ordering},
};

use crate::*;

mod loader;
mod os;

/// Determines what library to look up when [LazyFn::try_link] is called.
#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash, Debug)]
pub enum LinkType<'a> {
	/// Specifies a specialization for loading vulkan functions using vulkan loaders.
	Vulkan,
	/// Specifies a generalization for loading functions using native system loaders.
	System(&'a [&'a ffi::CStr]),
}

/// Fundamental data type of dylink.
///
/// This can be used safely without the dylink macro, however using the `dylink` macro should be preferred.
/// This structure can be used seperate from the dylink macro to check if the libraries exist before calling a dylink generated function.
pub struct LazyFn<'a, F: 'static + Sync + Send> {
	// It's imperative that LazyFn manages once, so that `LazyFn::try_link` is sound.
	pub(crate) state: AtomicBool,
	pub(crate) is_init: AtomicBool,
	// this is here to track the state of the instance during `LazyFn::try_link`.
	status: cell::RefCell<Option<error::DylinkError>>,
	// this exists so that `F` is considered thread-safe
	pub(crate) addr_ptr: AtomicPtr<F>,
	// The function to be called.
	// Non-function types can be stored, but obviously can't be called (call ops aren't overloaded).
	// The atomic pointer will always point to this
	pub(crate) addr: cell::UnsafeCell<F>,
	fn_name: &'a ffi::CStr,
	link_ty: LinkType<'a>,
}

impl<'a, F: 'static + Copy + Sync + Send> LazyFn<'a, F> {
	/// Initializes a `LazyFn` with a placeholder value `thunk`.
	/// # Panic
	/// Type `F` must be the same size as a [function pointer](fn) or `new` will panic.
	#[inline]
	pub const fn new(thunk: &'a F, fn_name: &'a ffi::CStr, link_ty: LinkType<'a>) -> Self {
		// In a const context this assert will be optimized out.
		assert!(mem::size_of::<FnPtr>() == mem::size_of::<F>());
		Self {
			state: AtomicBool::new(false),
			is_init: AtomicBool::new(true),
			addr_ptr: AtomicPtr::new(thunk as *const _ as *mut _),
			status: cell::RefCell::new(None),
			addr: cell::UnsafeCell::new(*thunk),
			fn_name,
			link_ty,
		}
	}

	/// If successful, stores address in current instance and returns a reference of the stored value.
	pub fn try_link(&'a self) -> Result<&'a F> {
		//lock spinlock
		while self
			.state
			.swap(self.is_init.load(Ordering::Acquire), Ordering::SeqCst)
		{
			core::hint::spin_loop()
		}

		if self.is_init.load(Ordering::Acquire) {
			let maybe = match self.link_ty {
				LinkType::Vulkan => unsafe { loader::vulkan_loader(self.fn_name) },
				LinkType::System(lib_list) => {
					let mut errors = alloc::vec::Vec::new();
					lib_list
						.iter()
						.find_map(|lib| {
							loader::system_loader(lib, self.fn_name)
								.or_else(|e| {
									errors.push(e);
									Err(())
								})
								.ok()
						})
						.ok_or_else(|| {
							let mut err = alloc::vec::Vec::new();
							for e in errors {
								err.push(e.to_string());
							}
							DylinkError::ListNotLoaded(err)
						})
				}
			};

			match maybe {
				Ok(addr) => {
					let addr_ptr = self.addr.get();
					unsafe {
						addr_ptr.write(mem::transmute_copy(&addr));
					}
					self.addr_ptr.store(addr_ptr as *mut F, Ordering::Release);
				}
				Err(err) => {
					let _ = self.status.replace(Some(err));
				}
			}
			// unlock spinlock
			self.is_init.store(false, Ordering::Release);
		}
		match (*self.status.borrow()).clone() {
			None => Ok(self.as_ref()),
			Some(err) => Err(err),
		}
	}

	#[inline]
	fn as_ref(&self) -> &F {
		unsafe {
			self.addr_ptr
				.load(Ordering::Relaxed)
				.as_ref()
				.unwrap_unchecked()
		}
	}
}

unsafe impl<F: 'static + Sync + Send> Send for LazyFn<'_, F> {}
unsafe impl<F: 'static + Sync + Send> Sync for LazyFn<'_, F> {}

impl<F: 'static + Copy + Sync + Send> core::ops::Deref for LazyFn<'_, F> {
	type Target = F;

	fn deref(&self) -> &Self::Target {
		self.as_ref()
	}
}
