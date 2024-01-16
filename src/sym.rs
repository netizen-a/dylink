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
	/// Returns the address of the symbol.
	#[inline]
	pub const fn as_ptr(self) -> *mut ffi::c_void {
		self.0 as _
	}
	/// Attempts to get the base address of the library.
	#[inline]
	pub fn image(self) -> Option<&'a img::Image> {
		unsafe { imp::base_addr(self.0).as_ref() }
	}
}