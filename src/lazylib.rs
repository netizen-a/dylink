use crate::loader;
use crate::loader::{LibHandle, Loader};
use crate::FnAddr;
use once_cell::sync::OnceCell;
use std::ffi::CStr;
use std::marker::PhantomData;
use std::sync::atomic::AtomicPtr;
#[cfg(feature="unload")]
use std::sync::Mutex;

// this wrapper struct is the bane of my existance...
#[derive(Debug)]
pub(crate) struct FnAddrWrapper(pub FnAddr);
unsafe impl Send for FnAddrWrapper {}

#[derive(Debug)]
pub struct LazyLib<'a, L: Loader<'a> = loader::System, const N: usize = 1> {
	libs: [&'static CStr; N],
	// library handles sorted by name
	pub(crate) hlib: OnceCell<LibHandle<'a, L::Data>>,
	// reset lock vector
	#[cfg(feature="unload")]
	pub(crate) rstl: Mutex<Vec<(&'static AtomicPtr<()>, FnAddrWrapper)>>,
	phtm: PhantomData<L>,
}

impl<'a, L: Loader<'a>, const N: usize> LazyLib<'a, L, N> {
	pub const fn new(libs: [&'static CStr; N]) -> Self {
		Self {
			libs,
			hlib: OnceCell::new(),
			#[cfg(feature="unload")]
			rstl: Mutex::new(Vec::new()),
			phtm: PhantomData,
		}
	}
	/// loads function from library synchronously and binds library handle internally to dylink.
	///
	/// If the library is already bound, the bound handle will be used for loading the function.
	pub unsafe fn find_sym(
		&self,
		sym: &'static CStr,
		_init: FnAddr,
		_atom: &'static AtomicPtr<()>,
	) -> crate::FnAddr
	where
		L::Data: 'static + Send,
	{
		let maybe_handle = self.hlib.get_or_try_init(|| {
			let mut handle;
			for lib_name in self.libs {
				handle = L::load_lib(lib_name);
				if !handle.is_invalid() {
					return Ok(handle);
				}
			}
			Err(())
		});
		if let Ok(lib_handle) = maybe_handle {
			#[cfg(feature="unload")]
			self.rstl.lock().unwrap().push((_atom, FnAddrWrapper(_init)));
			L::load_sym(&lib_handle, sym)
		} else {
			std::ptr::null()
		}
	}
}