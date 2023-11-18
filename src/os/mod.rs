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

#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct InnerLibrary(std::ptr::NonNull<ffi::c_void>);

// This function only works for executable images.
#[inline]
pub(crate) fn is_dangling(addr: *const Header) -> bool {
	unsafe { imp::base_addr(addr.cast_mut().cast()).is_err() }
}

// TODO: Next version bump this needs to be moved to a different module.
//
// Platform behavior:
//     MacOS   -> mach_header
//     Windows -> IMAGE_DOS_HEADER -> IMAGE_FILE_HEADER | IMAGE_OS2_HEADER | IMAGE_VXD_HEADER
//     Linux   -> ElfN_Ehdr
#[repr(C)]
pub struct Header {
	_data: [u8; 0],
	_marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}
