use std::ffi;
use std::io;
use std::os::windows::prelude::AsRawHandle;
use std::path;
use std::process;
//use std::ptr;
use std::sync::atomic;

use crate::Library;
//use crate::Sym;

use super::LibraryExt;
use super::c;
use super::SymbolHandler;

use std::os::windows::ffi::OsStrExt;

// only a single symbol handler may exist per process.
static HANDLER_EXISTS: atomic::AtomicBool = atomic::AtomicBool::new(false);

impl SymbolHandler {
	/// Constructs a SymbolHandler
	///
	/// **Note: Only one symbol handler may exist per process.**
	/// # Errors
	/// May error if another `SymbolHandler` instance is running.
	pub fn new<P: AsRef<path::Path>>(
		process: Option<&process::Child>,
		paths: &[P],
	) -> io::Result<Self> {
		if !HANDLER_EXISTS.swap(true, atomic::Ordering::SeqCst) {
			let hprocess = if let Some(child) = process {
				use std::os::windows::io::AsHandle;
				child.as_handle().as_raw_handle()
			} else {
				unsafe { c::GetCurrentProcess() }
			};

			let mut path_list = ffi::OsString::new();
			let mut first_path = true;
			for path in paths {
				if let Some(path_str) = path.as_ref().to_str() {
					if !first_path {
						path_list.push(";");
					}
					first_path = false;
					path_list.push(path_str);
				}
			}

			let usersearchpath: Vec<u16> = path_list
				.encode_wide()
				.chain(std::iter::once(0u16))
				.collect();
			let result = unsafe { c::SymInitializeW(hprocess, usersearchpath.as_ptr(), 0) };
			if result == 0 {
				Err(io::Error::last_os_error())
			} else {
				Ok(Self(hprocess))
			}
		} else {
			Err(io::Error::new(
				io::ErrorKind::AlreadyExists,
				"symbol handler already exists",
			))
		}
	}
    // untested. you probably shouldn't use this regardless.
    //pub unsafe fn delete_symbol(&self, library: &mut Library, symbol: &Sym) {
    //    c::SymDeleteSymbol(self.0, library.0 as c::ULONG64, ptr::null(), symbol as *const _ as _, 0);
    //}

}

impl Drop for SymbolHandler {
	fn drop(&mut self) {
		unsafe {
			c::SymCleanup(self.0);
		}
		HANDLER_EXISTS.store(false, atomic::Ordering::SeqCst)
	}
}

impl TryFrom<&Library> for SymbolHandler {
    type Error = io::Error;
    fn try_from(value: &Library) -> Result<Self, Self::Error> {
        value.path().and_then(|path| SymbolHandler::new(None, &[path]))
    }
}