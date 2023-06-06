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
#[derive(Debug)]
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
	pub(crate) const fn to_opaque<'b>(&self) -> LibHandle<'b, ffi::c_void> {
		LibHandle(self.0.cast(), PhantomData)
	}
	pub(crate) const fn from_opaque<'b>(a: &LibHandle::<ffi::c_void>) -> LibHandle::<'b, T> {
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

/// Used to specify a custom run-time linker loader for [LazyFn]
pub trait Loader {
	type Data;
	fn load_lib(lib_name: &'static ffi::CStr) -> LibHandle<'static, Self::Data>;
	fn load_sym(lib_handle: &LibHandle<'static, Self::Data>, fn_name: &'static ffi::CStr) -> FnAddr;
}





/// Default system linker used in [LazyFn]
#[cfg(not(wasm))]
pub struct System;

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

	impl Loader for System {
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
		use std::ffi::CStr;
		static LIB: lazylib::LazyLib<System, 1> = LazyLib::new(unsafe {[CStr::from_bytes_with_nul_unchecked(b"Kernel32.dll\0")]});
		extern crate self as dylink;
		#[dylink::dylink(library = LIB)]
		extern "stdcall" {
			fn SetLastError(_: u32);
		}

		// macro output: function
		#[dylink::dylink(library = LIB)]
		extern "C" {
			fn GetLastError() -> u32;
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

	impl Loader for System {
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
