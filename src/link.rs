// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
#![allow(clippy::upper_case_acronyms)]

use crate::*;
use std::marker::PhantomData;
use std::{ffi, mem, sync::RwLock};

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
	// This is basically a clone to an opaque handle
	fn to_opaque<'b>(&'a self) -> LibHandle<'b, ffi::c_void> {
		LibHandle(self.0.cast(), PhantomData)
	}
	fn from_opaque<'b>(a: &'a LibHandle::<ffi::c_void>) -> LibHandle::<'b, T> {
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
pub trait RTLinker {
	type Data;
	fn load_lib(lib_name: &ffi::CStr) -> LibHandle<'static, Self::Data>
	where
		Self::Data: Send + Sync;
	fn load_sym(lib_handle: &LibHandle<'static, Self::Data>, fn_name: &ffi::CStr) -> FnAddr
	where
		Self::Data: Send + Sync;

	/// loads library once across all calls and attempts to load the function.
	#[doc(hidden)]
	fn load_with(lib_name: &ffi::CStr, fn_name: &ffi::CStr) -> DylinkResult<FnAddr>
	where
		Self::Data: 'static + Send + Sync,
	{
		static DLL_DATA: RwLock<Vec<(ffi::CString, LibHandle<ffi::c_void>)>> =
			RwLock::new(Vec::new());

		let fn_addr: FnAddr;
		let lib_handle: LibHandle::<Self::Data>;
		let read_lock = DLL_DATA.read().unwrap();
		match read_lock.binary_search_by_key(&lib_name, |(k, _)| k) {
			Ok(index) => {
				lib_handle = LibHandle::from_opaque(&read_lock[index].1);
				fn_addr = Self::load_sym(&lib_handle, fn_name)
			}
			Err(index) => {
				mem::drop(read_lock);
				lib_handle = Self::load_lib(lib_name);
				if lib_handle.is_invalid() {
					return Err(DylinkError::LibNotLoaded(
						lib_name.to_string_lossy().into_owned(),
					));
				} else {
					fn_addr = Self::load_sym(&lib_handle, fn_name);
					DLL_DATA
						.write()
						.unwrap()
						.insert(index, (lib_name.to_owned(), lib_handle.to_opaque()));
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
}

/// Default system linker used in [LazyFn]
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

	impl RTLinker for System {
		type Data = ffi::c_void;
		#[cfg_attr(miri, track_caller)]
		#[inline]
		fn load_lib(lib_name: &ffi::CStr) -> LibHandle<'static, Self::Data>
		where
			Self::Data: 'static + Send + Sync,
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
		fn load_sym(
			lib_handle: &LibHandle<'static, Self::Data>,
			fn_name: &ffi::CStr,
		) -> crate::FnAddr
		where
			Self::Data: 'static + Send + Sync,
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
		fn load_lib(lib_name: &CStr) -> LibHandle<'static, Self::Data> {
			unsafe {
				let result = dlopen(lib_name.as_ptr(), RTLD_NOW | RTLD_LOCAL);
				LibHandle::from(result.as_ref())
			}
		}
		#[cfg_attr(miri, track_caller)]
		fn load_sym(lib_handle: &LibHandle<'static, Self::Data>, fn_name: &CStr) -> crate::FnAddr {
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
