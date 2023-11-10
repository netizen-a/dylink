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

#[inline]
pub(crate) fn is_dangling(addr: *const ffi::c_void) -> bool {
	unsafe {imp::base_addr(addr.cast_mut()).is_err()}
}
