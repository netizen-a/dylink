use std::marker;
use crate::sealed::Sealed;
use crate::img;
use std::ffi;

#[cfg(unix)]
use crate::os::unix as imp;
#[cfg(windows)]
use crate::os::windows as imp;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(C)]
pub struct Symbol<'a>(pub(crate) *mut ffi::c_void, pub(crate) marker::PhantomData<&'a ()>);
impl Sealed for Symbol<'_> {}

impl<'a> Symbol<'a> {
	/// Casts to a pointer of another type.
	#[inline]
	pub const fn cast<T>(self) -> *mut T {
		self.0 as _
	}
	/// Attempts to get the base address of the library.
	#[inline]
	pub fn image(self) -> Option<&'a img::Image> {
		unsafe { imp::base_addr(self.0).as_ref() }
	}
}