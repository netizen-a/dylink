use std::ffi;

pub struct Object {
    // Base address of the object.
    base_addr: *mut ffi::c_void,
}