// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
#[cfg_attr(docsrs, doc(cfg(unix)))]
#[cfg(any(unix, docsrs))]
pub mod unix;
#[cfg(windows)]
pub(crate) mod windows;

#[cfg(unix)]
use unix as imp;
#[cfg(windows)]
use windows as imp;

use std::ffi;

// an owned handle may not be null
pub(crate) type Handle = std::ptr::NonNull<ffi::c_void>;

// This function only works for executable images.
#[inline]
pub(crate) fn is_dangling(addr: *const Header) -> bool {
	unsafe { imp::base_addr(addr.cast_mut().cast()).is_err() }
}

// Platform behavior:
//     MacOS   -> mach_header
//     Windows -> ???
//     Linux   -> ???
#[repr(C)]
pub struct Header {
	_data: [u8; 0],
	_marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}
