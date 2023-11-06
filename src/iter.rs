use crate::img;
use crate::os;
use std::io;
use std::vec;

#[cfg(unix)]
use os::unix as imp;
#[cfg(windows)]
use os::windows as imp;

pub struct Images {
	inner: vec::IntoIter<img::Weak>,
}

// this impl block represents data coming from the global scope.
impl Images {
	pub fn now() -> io::Result<Self> {
		let inner = unsafe {
			imp::load_objects()?
				.into_iter()
		};
		Ok(Self { inner })
	}
}

impl<'a> Iterator for Images {
	type Item = img::Weak;
	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next()
	}
}
