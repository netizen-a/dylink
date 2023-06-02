// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use std::{
	cell,
	ffi::CStr,
	mem,
	sync::atomic::{AtomicPtr, Ordering},
};

use crate::{error, vulkan, DefaultLinker, DylinkResult};
use once_cell::sync::OnceCell;

/// Determines how to load the library when [LazyFn::try_link] is called.
#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash, Debug)]
pub enum LinkType<'a> {
	/// Specifies a specialization for loading vulkan functions using vulkan loaders.
	Vulkan,
	/// Specifies a generalization for loading functions.
	General(&'a [&'a CStr]),
}

/// Fundamental data type of dylink.
///
/// This can be used safely without the dylink macro, however using the `dylink` macro should be preferred.
/// The provided member functions can be used from the generated macro when `strip=true` is enabled.
#[derive(Debug)]
pub struct LazyFn<'a, F: Copy> {
	// It's imperative that LazyFn manages once, so that `LazyFn::try_link` is sound.
	pub(crate) once: OnceCell<()>,
	// this is here to track the state of the instance during `LazyFn::try_link`.
	//status: sync::OnceLock<error::DylinkError>,
	// this exists so that `F` is considered thread-safe
	pub(crate) addr_ptr: AtomicPtr<F>,
	// The function to be called.
	// mutating this data without locks is UB.
	pub(crate) addr: cell::UnsafeCell<F>,
	//pub(crate) init: F,
	fn_name: &'a CStr,
	link_ty: LinkType<'a>,
}

unsafe impl<F: Copy> Sync for LazyFn<'_, F> {}

impl<'a, F: Copy> LazyFn<'a, F> {
	/// Initializes a `LazyFn` with a placeholder value `thunk`.
	/// # Panic
	/// Type `F` must be the same size as a [function pointer](fn) or `new` will panic.
	#[inline]
	pub const fn new(thunk: &'a F, fn_name: &'a CStr, link_ty: LinkType<'a>) -> Self {
		// In a const context this assert will be optimized out.
		assert!(mem::size_of::<crate::FnPtr>() == mem::size_of::<F>());
		Self {
			addr_ptr: AtomicPtr::new(thunk as *const _ as *mut _),
			once: OnceCell::new(),
			//status: sync::OnceLock::new(),
			addr: cell::UnsafeCell::new(*thunk),
			//init: *thunk,
			fn_name,
			link_ty,
		}
	}

	/// Implicitly calls system defined linker loader, such as `GetProcAddress`, and `LoadLibraryExW`
	/// for windows, or `dlsym`, and `dlopen` for unix. This function is used by the
	/// [dylink](dylink_macro::dylink) macro by default.
	/// If successful, stores address in current instance and returns a reference of the stored value.
	///
	/// # Errors
	/// If the library fails to link, like if it can't find the library or function, then an error is returned.
	/// # Example
	/// ```rust
	/// # use dylink::dylink;
	/// #[dylink(name = "MyDLL.dll", strip = true)]
	/// extern "C" {
	///     fn foo();
	/// }
	///
	/// match foo.try_link() {
	///     Ok(func) => unsafe {func()},
	///     Err(err) => {
	///         println!("{err}")
	///     }
	/// }
	/// ```
	pub fn try_link(&self) -> DylinkResult<&F> {
		self.try_link_with::<DefaultLinker>()
	}

	/// Provides a generic argument to supply a user defined linker loader to load the library.
	/// If successful, stores address in current instance and returns a reference of the stored value.
	///
	/// # Errors
	/// If the library fails to link, like if it can't find the library or function, then an error is returned.
	pub fn try_link_with<L: crate::RTLinker>(&self) -> DylinkResult<&F>
	where
		L::Data: Send + Sync,
	{
		self.once
			.get_or_try_init(|| {
				match self.link_ty {
					LinkType::Vulkan => unsafe {
						vulkan::vulkan_loader(self.fn_name).ok_or(error::DylinkError::FnNotFound(
							self.fn_name.to_str().unwrap().to_owned(),
						))
					},
					LinkType::General(lib_list) => {
						let mut errors = vec![];
						lib_list
							.iter()
							.find_map(|lib| {
								L::load_with(lib, self.fn_name)
									.map_err(|e| errors.push(e))
									.ok()
							})
							.ok_or_else(|| match errors.len() {
								1 => errors[0].clone(),
								2..=usize::MAX => error::DylinkError::ListNotLoaded(
									errors.iter().map(|e| e.to_string() + "\n").collect(),
								),
								_ => unreachable!(),
							})
					}
				}
				.and_then(|addr| {
					unsafe {
						*self.addr.get() = mem::transmute_copy(&addr);
					}
					self.addr_ptr.store(self.addr.get(), Ordering::Release);
					Ok(())
				})
			})
			.and(Ok(self.load(Ordering::Acquire)))
	}

	#[inline]
	fn load(&self, order: Ordering) -> &F {
		unsafe { self.addr_ptr.load(order).as_ref().unwrap_unchecked() }
	}
	/// Consumes `LazyFn` and returns the contained value.
	///
	/// This is safe because passing self by value guarantees that no other threads are concurrently accessing `LazyFn`.
	pub fn into_inner(self) -> F {
		self.addr.into_inner()
	}
}

// should this be removed in favor of just calling load?

impl<F: Copy> std::ops::Deref for LazyFn<'_, F> {
	type Target = F;
	/// Dereferences the value atomically.
	fn deref(&self) -> &Self::Target {
		self.load(Ordering::Relaxed)
	}
}
