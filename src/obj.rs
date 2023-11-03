use std::ffi;


pub struct Object {
    // Base address of the object.
    pub(crate) base_addr: *mut ffi::c_void,
}

impl Object {
    //pub fn is_valid() -> bool;
}