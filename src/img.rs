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
		Self: Sized,
	{
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

// This function only works for executable images.
#[inline]
pub(crate) fn is_dangling(addr: *const Header) -> bool {
	unsafe { imp::base_addr(addr.cast_mut().cast()).is_null() }
}

// Platform behavior:
//     MacOS   -> mach_header | mach_header_64
//     Windows -> IMAGE_DOS_HEADER -> IMAGE_FILE_HEADER | IMAGE_OS2_HEADER | IMAGE_VXD_HEADER
//     Linux   -> Elf32_Ehdr | Elf64_Ehdr
#[repr(C)]
pub struct Header {
	_data: [u8; 0],
	_marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

impl Header {
	/// Returns the magic number of the image.
	pub fn magic(&self) -> &[u8] {
		let hdr = self as *const Header;
		let len: usize = if cfg!(windows) { 2 } else { 4 };
		unsafe { std::slice::from_raw_parts(hdr.cast::<u8>(), len) }
	}
	pub fn size(&self) -> io::Result<usize> {
		unsafe { imp::hdr_size(self) }
	}
}
