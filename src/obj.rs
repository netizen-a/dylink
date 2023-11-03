use std::{ffi, marker::PhantomData};


#[derive(PartialEq, Eq)]
pub struct Object<'a> {
    // Base address of the object.
    pub(crate) base_addr: *mut ffi::c_void,
    _marker: PhantomData<&'a ()>
}
unsafe impl Send for Object<'_>{}
unsafe impl Sync for Object<'_>{}

impl Object<'static> {
    pub unsafe fn from_ptr(base_addr: *mut ffi::c_void) -> Self {
        Self {
            base_addr,
            _marker: PhantomData
        }
    }
    pub unsafe fn into_ptr(self) -> *mut ffi::c_void {
        self.base_addr
    }
}

impl Object<'_> {
    pub fn is_valid() -> bool {
        false
    }
}