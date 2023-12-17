use crate::img;
use crate::os;
use crate::Library;
use std::path;
use std::ptr;

#[cfg(unix)]
use os::unix as imp;
#[cfg(windows)]
use os::windows as imp;

/// Represents an executable image.
///
/// This object can be obtained through either [`Images`](img::Images) or [`Library`].
#[derive(Debug, Clone)]
pub struct Weak {
	pub(crate) base_addr: *const img::Header,
	pub(crate) path_name: Option<path::PathBuf>,
}
impl crate::sealed::Sealed for Weak {}

impl Weak {
	/// Constructs a new `Weak`, without allocating any memory. Calling [`upgrade`] on the return value always gives [`None`].
	///
    /// [`upgrade`]: Weak::upgrade
    ///
    /// # Examples
    ///
    /// ```
    /// use dylink::Weak;
    ///
    /// let empty: Weak = Weak::new();
    /// assert!(empty.upgrade().is_none());
    /// ```
	pub const fn new() -> Self {
		Self {
			base_addr: ptr::null(),
			path_name: None,
		}
	}

	// The `Library::this` instance doesn't work well in doc tests.

	/// Attempts to upgrade the `Weak` pointer to a [`Library`], delaying dropping of the inner value if successful.
	///
	/// Returns [`None`] if the inner value has since been dropped.
	///
	/// # Examples
	///
	/// ```no_run
	/// use dylink::Library;
	///
	/// let this = Library::this();
	///
	/// let weak_this = Library::downgrade(&this).unwrap();
	///
	/// let strong_this: Option<Library> = weak_this.upgrade();
	/// assert!(strong_this.is_some());
	/// ```
	pub fn upgrade(&self) -> Option<Library> {
		unsafe { imp::InnerLibrary::from_ptr(self.base_addr.cast_mut()) }.map(Library)
	}

	/// Returns the base address of the image.
	///
	/// The pointer is only valid if there are some strong references to the image.
	/// The pointer may be dangling, unaligned or even [`null`] otherwise.
	///
	/// [`null`]: core::ptr::null "ptr::null"
	#[inline]
	pub fn to_ptr(&self) -> *const img::Header {
		unsafe { imp::base_addr(self.base_addr.cast_mut().cast()) }
	}
	/// Returns [`None`] if there is no associated image path, otherwise returns the path.
	///
	/// # Platform-specific Behavior
	///
	/// May return [`None`] on Linux if the image is the executable.
	#[inline]
	pub fn path(&self) -> Option<&path::Path> {
		self.path_name.as_deref()
	}
}
