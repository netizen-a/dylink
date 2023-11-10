use std::ffi;
#[cfg(unix)]
use crate::os::unix as imp;
#[cfg(windows)]
use crate::os::windows as imp;

use crate::Library;

// Represents an executable image. It essentially functions as a weak pointer and holds a base address.
pub struct Weak{
	pub(crate) base_addr: *mut ffi::c_void,
}
impl crate::sealed::Sealed for Weak {}

impl Weak {
	pub fn upgrade(&self) -> Option<Library> {
		unsafe {imp::dylib_upgrade(self.base_addr)}.map(Library)
	}
}



impl crate::Image for Weak {
	fn addr(&self) -> *mut ffi::c_void {
		self.base_addr
	}
}