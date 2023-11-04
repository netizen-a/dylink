use std::ffi;

// Fixme: replace with respective type.
type Header = ffi::c_void;


// The Object points to the base address.

/// Object is basically a weak pointer.
#[derive(PartialEq, Eq)]
#[repr(transparent)]
pub struct Object(pub(crate) *mut Header);

impl Object {
	// to implement upgrade I need a way of testing if the image is a library type.
	//pub fn upgrade();
}