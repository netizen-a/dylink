// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
#![allow(clippy::upper_case_acronyms)]

use crate::*;
use std::io::ErrorKind;
use std::marker::PhantomData;
use std::{ffi, mem, sync::RwLock};


// self documenting hidden trait
// TODO: add `Clone` trait on next version bump
#[doc(hidden)]
pub trait FnPtr: Copy {}
impl <T: Copy> FnPtr for T {}

// LibHandle is thread-safe because it's inherently immutable, therefore don't add mutable accessors.

/// Library handle for [RTLinker]
pub struct LibHandle<'a, T: ?Sized>(*const T, PhantomData<&'a T>);
unsafe impl<T> Send for LibHandle<'_, T> where T: Send {}
unsafe impl<T> Sync for LibHandle<'_, T> where T: Sync {}

impl<'a, T> LibHandle<'a, T> {
	#[inline]
	pub fn is_invalid(&self) -> bool {
		self.0.is_null()
	}
	pub fn as_ref(&self) -> Option<&T> {
		unsafe { self.0.as_ref() }
	}
	fn to_opaque<'b>(&self) -> LibHandle<'b, ffi::c_void> {
		LibHandle(self.0.cast(), PhantomData)
	}
	fn from_opaque<'b>(a: &LibHandle::<ffi::c_void>) -> LibHandle::<'b, T> {
		LibHandle::<T>(a.0.cast(), PhantomData)
	}
}

impl<'a, T> From<Option<&'a T>> for LibHandle<'a, T> {
	fn from(value: Option<&'a T>) -> Self {
		value
			.map(|r| Self((r as *const T).cast(), PhantomData))
			.unwrap_or(Self(std::ptr::null(), PhantomData))
	}
}

static DLL_DATA: RwLock<Vec<(&'static ffi::CStr, LibHandle<ffi::c_void>)>> =
			RwLock::new(Vec::new());

/// Used to specify a custom run-time linker loader for [LazyFn]
pub trait RTLinker {
	type Data;
	fn load_lib(lib_name: &'static ffi::CStr) -> LibHandle<'static, Self::Data>;
	fn load_sym(lib_handle: &LibHandle<'static, Self::Data>, fn_name: &'static ffi::CStr) -> FnAddr;
}

/// loads function from library synchronously and binds library handle internally to dylink.
/// 
/// If the library is already bound, the bound handle will be used for loading the function.
pub fn load_and_bind<L: RTLinker>(lib_name: &'static ffi::CStr, fn_name: &'static ffi::CStr) -> DylinkResult<FnAddr>
where
	L::Data: 'static + Send + Sync,
{
	let fn_addr: FnAddr;
	let lib_handle: LibHandle::<L::Data>;
	let read_lock = DLL_DATA.read().unwrap();
	match read_lock.binary_search_by_key(&lib_name, |(k, _)| k) {
		Ok(index) => {
			lib_handle = LibHandle::from_opaque(&read_lock[index].1);
			fn_addr = L::load_sym(&lib_handle, fn_name)
		}
		Err(index) => {
			mem::drop(read_lock);
			lib_handle = L::load_lib(lib_name);
			if lib_handle.is_invalid() {
				return Err(DylinkError::LibNotLoaded(
					lib_name.to_string_lossy().into_owned(),
				));
			} else {
				fn_addr = L::load_sym(&lib_handle, fn_name);
				DLL_DATA
					.write()
					.unwrap()
					.insert(index, (lib_name, lib_handle.to_opaque()));
			}
		}
	}
	if fn_addr.is_null() {
		Err(DylinkError::FnNotFound(
			fn_name.to_str().unwrap().to_owned(),
		))
	} else {
		Ok(fn_addr)
	}
}

/// unbinds handle synchronously from dylink and returns the handle.
/// 
/// This is safe because the library if found is not unloaded. If an uninitialized dylink generated function is called
/// after [`unbind`], the library will call [`load_lib`](RTLinker::load_lib) and bind another handle.
pub fn unbind<L: RTLinker>(lib_name: &'static ffi::CStr) -> Option<LibHandle<'static, L::Data>>
where
	L::Data: 'static + Send + Sync,
{
	let mut write_lock = DLL_DATA.write().unwrap();	
	match write_lock.binary_search_by_key(&lib_name, |(k, _)| k) {
		Ok(index) => {				
			Some(LibHandle::<L::Data>::from_opaque(&write_lock.remove(index).1))
		}
		Err(_) => None
	}
}

/// Default system linker used in [LazyFn]
pub struct System;

impl System {
	/// unbind and unload the library.
	/// 
	/// # Safety
	/// You should not call *any* functions associated with the library that have already been initialized.
	/// # Examples
	/// ```no_run
	/// use dylink::*;
	/// use std::ffi::CStr;
	///
	/// #[dylink(name = "Kernel32.dll")]
	/// extern "system" {
	///     fn GetLastError() -> u32;
	/// }
	/// fn main() {
	///     unsafe {
	///         println!("{}", GetLastError());
	///         let lib_name = CStr::from_bytes_with_nul(b"Kernel32.dll\0").unwrap();
	///         link::System::unload(lib_name).unwrap();
	///     }
	/// }
	/// ```
	#[cfg_attr(miri, track_caller)]
	pub unsafe fn unload(lib_name: &'static ffi::CStr) -> std::io::Result<()> {
		use std::ffi::{c_int, c_void};
		use std::io::Error;

		// windows and unix use the same type signature. how convenient :)
		extern "system" {
			#[cfg_attr(windows, link_name="FreeLibrary")]
			#[cfg_attr(unix, link_name="dlclose")]
			fn dlclose(hlibmodule: *mut c_void) -> c_int;
		}

		let ret: c_int;
		match unbind::<Self>(lib_name) {
			Some(lib_handle) => {
				let handle = lib_handle
					.as_ref()
					.map(|r| r as *const _ as *mut ffi::c_void)
					.unwrap();
				ret = dlclose(handle);
			}
			None => {
				return Err(Error::new(ErrorKind::NotFound, "could not find library handle"))
			}
		}
		let is_success = if cfg!(windows) {
			ret != 0
		} else if cfg!(unix) {
			ret == 0
		} else {
			unreachable!();
		};
		match is_success {
			true => Ok(()),
			false => Err(Error::last_os_error())
		}
	}
}

#[cfg(windows)]
mod win32 {
	use super::*;
	// The windows API conventions are kept deliberately, so it's easier to refer to references.

	use std::ffi;	
	use std::os::windows::raw::HANDLE;
	type HMODULE = HANDLE;
	type PCSTR = *const ffi::c_char;
	type PCWSTR = *const u16;
	const LOAD_LIBRARY_SEARCH_DEFAULT_DIRS: u32 = 0x00001000u32;
	const LOAD_LIBRARY_SAFE_CURRENT_DIRS: u32 = 0x00002000u32;
	extern "stdcall" {
		fn LoadLibraryExW(lplibfilename: PCWSTR, hfile: HANDLE, dwflags: u32) -> HMODULE;
		fn GetProcAddress(hmodule: HMODULE, lpprocname: PCSTR) -> crate::FnAddr;
	}

	impl RTLinker for System {
		type Data = ffi::c_void;
		#[cfg_attr(miri, track_caller)]
		#[inline]
		fn load_lib(lib_name: &'static ffi::CStr) -> LibHandle<'static, Self::Data>
		{
			let wide_str: Vec<u16> = lib_name
				.to_string_lossy()
				.encode_utf16()
				.chain(std::iter::once(0u16))
				.collect();
			let result = unsafe {
				// miri hates this function, but it works fine.
				LoadLibraryExW(
					wide_str.as_ptr().cast(),
					std::ptr::null_mut(),
					LOAD_LIBRARY_SEARCH_DEFAULT_DIRS | LOAD_LIBRARY_SAFE_CURRENT_DIRS,
				)
			};
			LibHandle::from(unsafe { result.as_ref() })
		}
		#[cfg_attr(miri, track_caller)]
		#[inline]
		fn load_sym(
			lib_handle: &LibHandle<'static, Self::Data>,
			fn_name: &'static ffi::CStr,
		) -> crate::FnAddr
		{
			unsafe {
				GetProcAddress(
					lib_handle
						.as_ref()
						.map(|r| r as *const _ as *mut ffi::c_void)
						.unwrap_or(std::ptr::null_mut()),
					fn_name.as_ptr().cast(),
				)
			}
		}
	}

	

	#[cfg(not(miri))]
	#[test]
	fn test_win32_macro_linker() {
		extern crate self as dylink;
		#[dylink::dylink(name = "Kernel32.dll", strip = true, linker=System)]
		extern "stdcall" {
			fn SetLastError(_: u32);
		}

		// macro output: function
		#[dylink::dylink(name = "Kernel32.dll", strip = false, linker=System)]
		extern "C" {
			fn GetLastError() -> u32;
		}

		unsafe {
			// static variable has crappy documentation, but can be use for library induction.
			match SetLastError.try_link() {
				Ok(f) => f(53),
				Err(e) => panic!("{}", e),
			}
			assert_eq!(GetLastError(), 53);
		}
	}
}

#[cfg(unix)]
mod unix {
	use std::ffi::{c_char, c_int, c_void, CStr};

	use super::*;

	const RTLD_NOW: c_int = 0x2;
	const RTLD_LOCAL: c_int = 0;
	extern "C" {
		fn dlopen(filename: *const c_char, flag: c_int) -> *mut c_void;
		fn dlsym(handle: *mut c_void, symbol: *const c_char) -> crate::FnAddr;
	}

	impl RTLinker for System {
		type Data = c_void;
		#[cfg_attr(miri, track_caller)]
		#[inline]
		fn load_lib(lib_name: &'static CStr) -> LibHandle<'static, Self::Data> {
			unsafe {
				let result = dlopen(lib_name.as_ptr(), RTLD_NOW | RTLD_LOCAL);
				LibHandle::from(result.as_ref())
			}
		}
		#[cfg_attr(miri, track_caller)]
		#[inline]
		fn load_sym(lib_handle: &LibHandle<'static, Self::Data>, fn_name: &'static CStr) -> crate::FnAddr {
			unsafe {
				dlsym(
					lib_handle
						.as_ref()
						.map(|r| r as *const _ as *mut c_void)
						.unwrap_or(std::ptr::null_mut()),
					fn_name.as_ptr(),
				)
			}
		}
	}
	
}
