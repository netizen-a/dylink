// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
use std::{cell, ffi, mem, sync::{self, atomic::{AtomicPtr, Ordering}}};

use crate::*;

mod loader;

/// Determines what library to look up when [LazyFn::load] is called.
#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash, Debug)]
pub enum LinkType {
	/// Specialization for loading vulkan functions
	Vulkan,
	// TODO: change name from `Normal` to `System` in next minor release
	/// Generalization for loading normal functions.
	Normal(&'static [&'static str]),
}

/// Fundamental data type of dylink.
///
/// This can be used safely without the dylink macro, however using the `dylink` macro should be preferred.
/// This structure can be used seperate from the dylink macro to check if the libraries exist before calling a dylink generated function.
pub struct LazyFn<F: 'static> {
	// It's imperative that LazyFn manages once, so that `LazyFn::load` is sound.
	pub(crate) once: sync::Once,
	// this is here to track the state of the instance.
	status: cell::UnsafeCell<Option<error::DylinkError>>,
	// The function to be called.
	// Non-function types can be stored, but obviously can't be called (call ops aren't overloaded).
	pub(crate) addr_ptr: AtomicPtr<F>,
	addr: cell::UnsafeCell<Option<F>>,
}

impl<F: 'static + Copy> LazyFn<F> {
	/// Initializes a `LazyFn` with a placeholder value `thunk`.
	/// # Panic
	/// Type `F` must be the same size as a [function pointer](fn) or `new` will panic.
	#[inline]
	pub const fn new(thunk: AtomicPtr<F>) -> Self {
		// In a const context this assert will be optimized out.
		assert!(mem::size_of::<FnPtr>() == mem::size_of::<F>());
		Self {
			addr_ptr: thunk,
			once: sync::Once::new(),
			status: cell::UnsafeCell::new(None),
			addr: cell::UnsafeCell::new(None)
		}
	}

	/// If successful, stores address in current instance and returns a reference to the stored value.
	pub fn load(&self, fn_name: &'static ffi::CStr, link_ty: LinkType) -> Result<F> {
		let str_name: &'static str = fn_name.to_str().unwrap();
		self.once.call_once(|| unsafe {
			let maybe = match link_ty {
				LinkType::Vulkan => loader::vulkan_loader(str_name),
				LinkType::Normal(lib_list) => {
					let default_error = {
						let (subject, kind) = if lib_list.len() > 1 {
							(None, ErrorKind::ListNotFound)
						} else {
							(Some(lib_list[0]), ErrorKind::LibNotFound)
						};
						error::DylinkError::new(subject, kind)
					};
					let mut result = Err(default_error);
					for lib_name in lib_list {
						match loader::system_loader(ffi::OsStr::new(lib_name), str_name) {
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
					let addr_ptr = self.addr.get();
					addr_ptr.write(Some(mem::transmute_copy(&addr)));
					self.addr_ptr.store(mem::transmute(addr_ptr), Ordering::Relaxed);
				}
				Err(err) => {
					self.status.get().write(Some(err));
				}
			}
		});
		// `call_once` is blocking, so `self.status` is read-only
		// by this point. Race conditions shouldn't occur.
		match unsafe { (*self.status.get()).clone() } {
			None => Ok(*self.as_ref()),
			Some(err) => Err(err),
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
		unsafe { self.addr_ptr.load(Ordering::Relaxed).as_ref().unwrap_unchecked() }
	}
}
