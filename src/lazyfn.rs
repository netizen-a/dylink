use std::{cell, mem, sync, ffi};

use crate::{error::*, loader::*, FnPtr, Result, VK_INSTANCE};

#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash, Debug)]
pub enum LinkType {
	OpenGL,
	Vulkan,
	Normal(&'static [u8]),
}

// This can be used safely without the dylink macro.
// `F` can be anything as long as it's the size of a function pointer
pub struct LazyFn<F: 'static> {
	name:    &'static [u8],
	addr:    cell::UnsafeCell<F>,
	link_ty: LinkType,
	once:    sync::Once,
	status:  cell::UnsafeCell<Option<ErrorKind>>,
}

unsafe impl<F: 'static> Sync for LazyFn<F> {}

impl<F: 'static> LazyFn<F> {
	/// Initializes a `LazyFn` object with all the necessary information for `LazyFn::link` to work.
	/// # Panic
	/// The provided slice, `name`, must be nul-terminated and not contain any interior nul bytes, 
	/// if not the function will panic.
	/// 
	/// Thunk must be the same size as `FnPtr`.
	#[inline]
	pub const fn new(name: &'static [u8], thunk: F, link_ty: LinkType) -> Self {
		// this check is optimized out if called in a const context
		assert!(matches!(name, [.., 0]));
		assert!(mem::size_of::<FnPtr>() == mem::size_of::<F>());
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
		// this is safe because nul is checked in `LazyFn::new`.
		let fn_name = unsafe {ffi::CStr::from_bytes_with_nul_unchecked(self.name)};
		self.once.call_once(|| unsafe {
			let maybe = match self.link_ty {
				LinkType::Vulkan => {
					let read_lock = VK_INSTANCE.read().expect("failed to get read lock");
					// check other instances if fails in case one has a higher available version number
					match read_lock
						.iter()
						.find_map(|instance| vkloader(fn_name, Some(instance)).ok())
					{
						Some(addr) => Ok(addr),
						None => vkloader(fn_name, None),
					}
				}
				LinkType::OpenGL => glloader(fn_name),
				LinkType::Normal(lib_name) => loader(ffi::CStr::from_bytes_with_nul_unchecked(lib_name), fn_name),
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
			Some(kind) => Err(DylinkError::new(fn_name.to_str().unwrap(), kind)),
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
