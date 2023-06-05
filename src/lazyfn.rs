// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use std::{
	cell,
	ffi::CStr,
	mem,
	sync::atomic::{AtomicPtr, Ordering}, marker::PhantomData,
};

use crate::{error, vk, link, DylinkResult, FnAddr};
use once_cell::sync::OnceCell;




/// Determines how to load the library when [LazyFn::try_link] is called.
#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash, Debug)]
pub enum LinkType<'a> {
	/// Specifies a specialization for loading vulkan functions using vulkan loaders.
	Vulkan,
	/// Specifies a generalization for loading functions.
	General(&'a [&'static CStr]),
}

/// Fundamental data type of dylink.
///
/// This can be used safely without the dylink macro, however using the `dylink` macro should be preferred.
/// The provided member functions can be used from the generated macro when `strip=true` is enabled.
#[derive(Debug)]
pub struct LazyFn<'a, F: link::FnPtr, L: link::RTLinker = link::System> {
	// It's imperative that LazyFn manages once, so that `LazyFn::try_link` is sound.
	// SAFETY: once is not allowed to be reset, because it can break references
	pub(crate) once: OnceCell<F>,
	// this exists so that `F` is considered thread-safe
	pub(crate) addr_ptr: AtomicPtr<F>,
	// The function to be called.
	// SAFETY: This should only be accessed atomically or when thread is blocking.
	pub(crate) addr: cell::UnsafeCell<F>,
	fn_name: &'static CStr,
	link_ty: LinkType<'a>,
	phantom: PhantomData<L>,
}

unsafe impl<F: link::FnPtr, L: link::RTLinker> Sync for LazyFn<'_, F, L> {}

impl<'a, F: link::FnPtr, L: link::RTLinker> LazyFn<'a, F, L> {
	/// Initializes a `LazyFn` with a placeholder value `thunk`.
	/// # Panic
	/// Type `F` must be the same size as a [function pointer](fn) or `new` will panic.
	#[inline]
	pub const fn new(thunk: &'a F, fn_name: &'static CStr, link_ty: LinkType<'a>) -> Self {
		assert!(mem::size_of::<crate::FnAddr>() == mem::size_of::<F>());
		Self {
			addr_ptr: AtomicPtr::new(thunk as *const _ as *mut _),
			once: OnceCell::new(),
			addr: cell::UnsafeCell::new(*thunk),
			fn_name,
			link_ty,
			phantom: PhantomData,
		}
	}

	/// Calls the run-time linker loader, and tries to link the function.
	/// If successful, stores address in current instance and returns a copy of the stored value.
	///
	/// # Errors
	/// If the library or function can't be found, then an error value is returned.
	/// 
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
	pub fn try_link(&self) -> DylinkResult<F>
	where
		L::Data: 'static + Send + Sync,
	{
		self.once
			.get_or_try_init(|| {
				match self.link_ty {
					LinkType::Vulkan => unsafe {
						let addr: FnAddr = vk::vulkan_loader(self.fn_name);
						if addr.is_null() {
							Err(error::DylinkError::FnNotFound(
								self.fn_name.to_str().unwrap().to_owned(),
							))
						} else {
							Ok(addr)
						}
					},
					LinkType::General(lib_list) => {
						let mut errors = vec![];
						lib_list
							.iter()
							.find_map(|lib| {
								link::load_and_bind::<L>(lib, self.fn_name)
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
				.map(|raw_addr| unsafe {
					let addr = self
						.addr
						.get()
						.replace(mem::transmute_copy::<_, F>(&raw_addr));
					self.addr_ptr.store(self.addr.get(), Ordering::Release);
					addr
				})
			})
			.and(Ok(unsafe {*self.load(Ordering::Acquire)}))
	}

	/// loads a value from the object.
	/// 
	/// `load` takes an [`Ordering`] argument which describes the memory ordering
	/// of this operation. Possible values are [`SeqCst`](Ordering::SeqCst), [`Acquire`](Ordering::Acquire) and [`Relaxed`](Ordering::Relaxed).
	/// # Panics
	/// Panics if `order` is [`Release`](Ordering::Release) or [`AcqRel`](Ordering::AcqRel).
	#[inline]
	fn load(&self, order: Ordering) -> *mut F {
		self.addr_ptr.load(order)
	}
	/// Consumes `LazyFn` and returns the contained value.
	///
	/// This is safe because passing self by value guarantees that no other threads are concurrently accessing `LazyFn`.
	pub fn into_inner(self) -> F {
		self.addr.into_inner()
	}
}

// This will always return a valid reference, but not always the same reference
impl<F: link::FnPtr, L: link::RTLinker> std::ops::Deref for LazyFn<'_, F, L> {
	type Target = F;
	/// Dereferences the value atomically.
	fn deref(&self) -> &Self::Target {
		unsafe {self.load(Ordering::Relaxed).as_ref().unwrap_unchecked()}
	}
}
