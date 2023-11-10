use std::{ffi, path, io};
#[cfg(unix)]
use crate::os::unix as imp;
#[cfg(windows)]
use crate::os::windows as imp;

use crate::Library;

/// Represents an executable image.
///
/// This object can be obtained through either [`Images`](crate::iter::Images) or [`Library`].
pub struct Weak{
	pub(crate) base_addr: *mut ffi::c_void,
	pub(crate) path_name: Option<path::PathBuf>,
}
impl crate::sealed::Sealed for Weak {}

impl Weak {
	pub fn upgrade(&self) -> Option<Library> {
		unsafe {imp::dylib_upgrade(self.base_addr)}.map(Library)
	}
}



impl crate::Image for Weak {
	#[inline]
	fn addr(&self) -> *mut ffi::c_void {
		self.base_addr
	}
	#[inline]
	fn path(&self) -> io::Result<path::PathBuf> {
		match self.path_name {
			Some(ref val) => Ok(val.clone()),
			None => Err(io::Error::new(io::ErrorKind::NotFound, "No path available")),
		}
	}
}