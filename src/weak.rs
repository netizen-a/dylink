use crate::img;
use crate::os;
use crate::Library;
use std::path;

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
	#[inline]
	pub fn to_ptr(&self) -> *const img::Header {
		if img::is_dangling(self.base_addr) {
			std::ptr::null()
		} else {
			self.base_addr
		}
	}
	#[inline]
	pub fn path(&self) -> Option<&path::PathBuf> {
		self.path_name.as_ref()
	}
}
