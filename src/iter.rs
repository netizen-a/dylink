use crate::obj::Object;
use crate::os;
use std::io;
use std::vec;

#[cfg(unix)]
use os::unix as imp;
#[cfg(windows)]
use os::windows as imp;

pub struct Objects<'a> {
	inner: vec::IntoIter<Object<'a>>,
}

impl Objects<'static> {
	pub fn now() -> io::Result<Self> {
		Ok(Self {
			inner: unsafe { imp::load_objects()?.into_iter() },
		})
	}
}

impl<'a> Iterator for Objects<'a> {
	type Item = Object<'a>;
	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next()
	}
}
