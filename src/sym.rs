use std::marker;
use crate::sealed::Sealed;
use crate::img;

#[cfg(unix)]
use crate::os::unix as imp;
#[cfg(windows)]
use crate::os::windows as imp;

#[repr(C)]
pub struct Symbol {
	_data: [u8; 0],
	_marker: marker::PhantomData<(*mut u8, marker::PhantomPinned)>,
}
impl Sealed for Symbol {}

impl Symbol {
	/// Attempts to get the base address of the library.
	#[inline]
	pub fn image<'a>(this: *const Symbol) -> Option<&'a img::Image> {
		unsafe { imp::base_addr(this.cast()).as_ref() }
	}
}