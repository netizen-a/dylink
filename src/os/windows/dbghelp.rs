use core::slice;
use std::ffi;
use std::io;
use std::mem;
use std::os::windows::prelude::*;
use std::path;
use std::process;
use std::ptr;
use std::sync::atomic;

use crate::Library;
use crate::Sym;

use super::c;
use super::SymbolHandler;

use std::os::windows::ffi::OsStrExt;
use std::os::windows::ffi::OsStringExt;

/// documentation: <https://learn.microsoft.com/en-us/windows/win32/api/dbghelp/ns-dbghelp-symbol_info>
#[derive(Debug)]
pub struct SymbolInfo {
	pub typeindex: c::ULONG,
	pub index: c::ULONG,
	pub size: c::ULONG,
	pub modbase: c::ULONG64,
	pub flags: c::ULONG,
	pub value: c::ULONG64,
	pub address: *const Sym,
	pub register: c::ULONG,
	pub scope: c::ULONG,
	pub tag: c::ULONG,
	pub name: ffi::OsString,
}

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
			let result = unsafe {
				c::SymSetOptions(c::SYMOPT_UNDNAME | c::SYMOPT_DEFERRED_LOADS);
				c::SymInitializeW(hprocess, usersearchpath.as_ptr(), 1)
			};
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
	pub fn symbol_info(&self, symbol: &Sym) -> io::Result<SymbolInfo> {
		let mut displacement: c::DWORD64 = 0;
		let address: c::DWORD64 = symbol as *const Sym as c::DWORD64;
		let mut buffer = vec![
			0u8;
			mem::size_of::<c::SYMBOL_INFOW>()
				+ c::MAX_SYM_NAME as usize * mem::size_of::<u16>()
		];

		let symbol_info: &mut c::SYMBOL_INFOW =
			unsafe { (buffer.as_mut_ptr() as *mut c::SYMBOL_INFOW).as_mut() }.unwrap();

		unsafe {
			symbol_info.sizeofstruct = mem::size_of::<c::SYMBOL_INFOW>() as c::ULONG;
			symbol_info.maxnamelen = c::MAX_SYM_NAME;

			if c::SymFromAddrW(self.0, address, &mut displacement, symbol_info) == 0 {
				Err(io::Error::last_os_error())
			} else {
				let name_slice = slice::from_raw_parts(
					ptr::addr_of!(symbol_info.name) as *const _,
					symbol_info.namelen as usize,
				);
				let info = SymbolInfo {
					typeindex: symbol_info.typeindex,
					index: symbol_info.index,
					size: symbol_info.size,
					modbase: symbol_info.modbase,
					flags: symbol_info.flags,
					value: symbol_info.value,
					address: symbol_info.address as *const Sym,
					register: symbol_info.register,
					scope: symbol_info.scope,
					tag: symbol_info.tag,
					name: ffi::OsString::from_wide(name_slice),
				};
				Ok(info)
			}
		}
	}

	pub fn map_modules<F>(&self, mut f: F) -> io::Result<()>
	where
		F: FnMut(&ffi::OsStr, &Library) -> bool,
	{
		unsafe extern "system-unwind" fn callback<F: FnMut(&ffi::OsStr, &Library) -> bool>(
			module_name: c::PCWSTR,
			base_of_dll: c::DWORD64,
			user_context: *mut ffi::c_void,
		) -> c::BOOL {
			let len = crate::os::wcslen(module_name);
			let raw_wide = std::slice::from_raw_parts(module_name, len);
			let wide_string = ffi::OsString::from_wide(raw_wide);
			let f = (user_context as *mut F).as_mut().unwrap_unchecked();
			let lib = mem::ManuallyDrop::new(Library(base_of_dll as *mut _));
			f(&wide_string, &*lib) as c::BOOL
		}
		unsafe {
			if 0 != c::SymEnumerateModulesW64(self.0, callback::<F>, &mut f as *mut _ as *mut _) {
				Ok(())
			} else {
				Err(io::Error::last_os_error())
			}
		}
	}

	// The following functions have a reciever parameter,
	// so they can only act on the same thread as the symbol handler.

	pub fn set_options(&mut self, options: c::DWORD) -> c::DWORD {
		unsafe { c::SymSetOptions(options) }
	}
	pub fn get_options(&self) -> c::DWORD {
		unsafe { c::SymGetOptions() }
	}
}

impl Drop for SymbolHandler {
	fn drop(&mut self) {
		unsafe {
			c::SymCleanup(self.0);
		}
		HANDLER_EXISTS.store(false, atomic::Ordering::SeqCst)
	}
}

impl AsHandle for SymbolHandler {
	fn as_handle(&self) -> BorrowedHandle<'_> {
		unsafe { BorrowedHandle::borrow_raw(self.0) }
	}
}
