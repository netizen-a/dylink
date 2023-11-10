use crate::os;
use crate::weak;
use std::io;
use std::iter::FusedIterator;
use std::vec;

#[cfg(unix)]
use os::unix as imp;
#[cfg(windows)]
use os::windows as imp;

// This is an iterator and not a vector because the data should be assumed stale.
#[derive(Debug, Clone)]
pub struct Images {
	inner: vec::IntoIter<weak::Weak>,
}

impl Images {
	/// Takes a snapshot of executable images currently loaded into memory.
	pub fn now() -> io::Result<Self> {
		let inner = unsafe { imp::load_objects()?.into_iter() };
		Ok(Self { inner })
	}
}

impl Iterator for Images {
	type Item = weak::Weak;
	#[inline]
	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next()
	}
	#[inline]
	fn size_hint(&self) -> (usize, Option<usize>) {
		self.inner.size_hint()
	}
	#[inline]
	fn count(self) -> usize
		where
			Self: Sized, {
		self.inner.count()
	}
}

impl DoubleEndedIterator for Images {
	#[inline]
	fn next_back(&mut self) -> Option<Self::Item> {
		self.inner.next_back()
	}
}

impl ExactSizeIterator for Images {
	#[inline]
	fn len(&self) -> usize {
		self.inner.len()
	}
}

impl FusedIterator for Images {}