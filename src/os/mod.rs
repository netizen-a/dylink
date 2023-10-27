// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
#[doc(cfg(unix))]
#[cfg(any(unix, docsrs))]
pub mod unix;
#[cfg(windows)]
pub(crate) mod windows;

// an owned handle may not be null
pub(crate) type Handle = std::ptr::NonNull<std::ffi::c_void>;
