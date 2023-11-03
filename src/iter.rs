use crate::os;
use std::ffi;
use std::io;

#[cfg(unix)]
use os::unix as imp;
#[cfg(windows)]
use os::windows as imp;

pub struct Objects {
    inner: Vec<*mut ffi::c_void>
}

impl Objects {
    pub fn now() -> io::Result<Self> {
        Ok(Self{
            inner: unsafe {imp::load_objects()?}
        })
    }
}