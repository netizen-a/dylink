use crate::img;
use crate::os;
use crate::Library;
use std::{io, path};

#[cfg(unix)]
use os::unix as imp;
#[cfg(windows)]
use os::windows as imp;

/// Represents an executable image.
///
/// This object can be obtained through either [`Images`](img::Images) or [`Library`].
#[derive(Debug, Clone)]
pub struct Weak {
	pub(crate) base_addr: *const img::Header,
	pub(crate) path_name: Option<path::PathBuf>,
}
impl crate::sealed::Sealed for Weak {}

impl Weak {
	pub fn upgrade(&self) -> Option<Library> {
		unsafe { imp::InnerLibrary::from_ptr(self.base_addr.cast_mut()) }.map(Library)
	}

	/// Returns the base address of the image.
	///
	/// The pointer is only valid if there are some strong references to the image.
	/// The pointer may be dangling, unaligned or even [`null`] otherwise.
	///
	/// [`null`]: core::ptr::null "ptr::null"
	#[inline]
	pub fn to_ptr(&self) -> *const img::Header {
		if img::is_dangling(self.base_addr) {
			std::ptr::null()
		} else {
			self.base_addr
		}
	}
	#[inline]
	pub fn path(&self) -> io::Result<path::PathBuf> {
		match self.path_name {
			Some(ref val) => Ok(val.clone()),
			None => Err(io::Error::new(io::ErrorKind::NotFound, "No path available")),
		}
	}
}

