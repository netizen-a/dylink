use crate::obj::Object;
use crate::os;
use std::ffi;
use std::io;
use std::vec;

#[cfg(unix)]
use os::unix as imp;
#[cfg(windows)]
use os::windows as imp;

pub struct Objects {
	inner: vec::IntoIter<*mut ffi::c_void>,
}

impl Objects {
	pub fn now() -> io::Result<Self> {
		Ok(Self {
			inner: unsafe { imp::load_objects()?.into_iter() },
		})
	}
}

impl Iterator for Objects {
	type Item = Object;
	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next().map(|base_addr| Object{base_addr})
	}
}