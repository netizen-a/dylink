use std::ffi;

use crate::Library;

// Represents an executable image. It essentially functions as a weak pointer and holds a base address.
pub struct Weak{
	pub(crate) base_addr: *mut ffi::c_void,
}
impl crate::sealed::Sealed for Weak {}

pub trait Image: crate::sealed::Sealed {
	fn base_addr(&self) -> *mut ffi::c_void;
}

impl Image for Weak {
	fn base_addr(&self) -> *mut ffi::c_void {
		self.base_addr
	}
}

//impl Image {
// to implement upgrade I need a way of testing if the image is a library type.
//pub fn upgrade();
//pub fn name();
//pub fn base_addr();
//}