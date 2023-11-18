use crate::os;
use crate::Library;
use std::{io, path};

#[cfg(unix)]
use os::unix as imp;
#[cfg(windows)]
use os::windows as imp;

/// Represents an executable image.
///
/// This object can be obtained through either [`Images`](crate::iter::Images) or [`Library`].
#[derive(Debug, Clone)]
pub struct Weak {
	pub(crate) base_addr: *const os::Header,
	pub(crate) path_name: Option<path::PathBuf>,
}
impl crate::sealed::Sealed for Weak {}

impl Weak {
	pub fn upgrade(&self) -> Option<Library> {
		unsafe { imp::InnerLibrary::from_ptr(self.base_addr.cast_mut()) }.map(Library)
	}
}

impl crate::Image for Weak {
	#[inline]
	fn to_ptr(&self) -> *const os::Header {
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