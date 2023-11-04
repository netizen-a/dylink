use crate::obj::Object;
use crate::os;
use std::io;
use std::vec;

#[cfg(unix)]
use os::unix as imp;
#[cfg(windows)]
use os::windows as imp;

pub struct Objects {
	inner: vec::IntoIter<Object>,
}

// this impl block represents data coming from the global scope.
impl Objects {
	pub fn now() -> io::Result<Self> {
		let inner = unsafe { imp::load_objects()?
			.into_iter()
			.map(Object)
			.collect::<Vec<Object>>()
			.into_iter()
		};
		Ok(Self {
			inner,
		})
	}
}

impl<'a> Iterator for Objects {
	type Item = Object;
	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next()
	}
}
