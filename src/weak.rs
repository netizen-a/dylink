#[cfg(unix)]
use crate::os::unix as imp;
#[cfg(windows)]
use crate::os::windows as imp;
use std::{ffi, io, path};
use crate::os;

use crate::Library;

/// Represents an executable image.
///
/// This object can be obtained through either [`Images`](crate::iter::Images) or [`Library`].
pub struct Weak {
	pub(crate) base_addr: *const ffi::c_void,
	pub(crate) path_name: Option<path::PathBuf>,
}
impl crate::sealed::Sealed for Weak {}

impl Weak {
	pub fn upgrade(&self) -> Option<Library> {
		unsafe { imp::dylib_upgrade(self.base_addr.cast_mut()) }.map(Library)
	}
}

impl crate::Image for Weak {
	#[inline]
	fn as_ptr(&self) -> *const ffi::c_void {
		if os::is_dangling(self.base_addr) {
			std::ptr::null()
		} else {
			self.base_addr
		}
	}
	#[inline]
	fn path(&self) -> io::Result<path::PathBuf> {
		match self.path_name {
			Some(ref val) => Ok(val.clone()),
			None => Err(io::Error::new(io::ErrorKind::NotFound, "No path available")),
		}
	}
}
