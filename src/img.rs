use crate::os;
use crate::weak;
use std::io;
use std::iter::FusedIterator;
use std::path;
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

impl From<Vec<weak::Weak>> for Images {
	fn from(value: Vec<weak::Weak>) -> Self {
		Self {
			inner: value.into_iter(),
		}
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

/// An opaque object representing an executable image.
///
/// # Platform behavior
///
/// The following are the expected headers the image may represent per each platform.
///
/// | Platform | Headers                                              |
/// | -------- | ---------------------------------------------------- |
/// | MacOS    | mach_header, mach_header_64                          |
/// | Windows  | IMAGE_DOS_HEADER, IMAGE_OS2_HEADER, IMAGE_VXD_HEADER |
/// | Linux    | Elf32_Ehdr, Elf64_Ehdr                               |
#[repr(C)]
pub struct Image {
	_data: [u8; 0],
	_marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

impl Image {
	/// Returns the path to the image.
	///
	/// # Security
	///
	/// If the Image comes from [`Library::this`](crate::Library) then this function should heed the same
	/// security implications as [`current_env`](std::env::current_exe).
	pub fn path(&self) -> io::Result<path::PathBuf> {
		unsafe { imp::hdr_path(self as *const Image) }
	}

	/// Converts this Image to a byte slice.
	pub fn to_bytes(&self) -> io::Result<&[u8]> {
		let len = unsafe { imp::hdr_size(self)? };
		let data = self as *const Image as *const u8;
		let slice = unsafe { std::slice::from_raw_parts(data, len) };
		Ok(slice)
	}
}