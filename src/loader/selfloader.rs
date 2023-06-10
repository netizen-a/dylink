// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
use super::*;
use crate::os::*;
use std::ffi::CStr;

#[cfg(windows)]
impl <'a> Loader<'a> for SelfLoader {
    type Data = std::ffi::c_void;

    fn load_lib(lib_name: &'static CStr) -> LibHandle<'a, Self::Data> {
        // FIXME: when `CStr::is_empty` is stable, replace `to_bytes().is_empty()`.
        if lib_name.to_bytes().is_empty() {
            LibHandle::from(unsafe {win32::GetModuleHandleW(std::ptr::null_mut()).as_ref()})
        } else {
            let wide_str: Vec<u16> = lib_name
				.to_string_lossy()
				.encode_utf16()
				.chain(std::iter::once(0u16))
				.collect();
            LibHandle::from(unsafe {win32::GetModuleHandleW(wide_str.as_ptr()).as_ref()})
        }
    }
    fn load_sym(
        lib_handle: &LibHandle<'a, Self::Data>,
        fn_name: &CStr
    ) -> FnAddr {
        unsafe {
            dlsym(lib_handle
                .as_ref()
                .map(|r| r as *const _ as *mut ffi::c_void)
                .unwrap_or(std::ptr::null_mut()), fn_name.as_ptr())
        }
    }
}
#[cfg(unix)]
impl <'a> Loader<'a> for SelfLoader {
    // dummy data type
    type Data = i8;

    fn load_lib(_: &CStr) -> LibHandle<'a, Self::Data> {
        // just needs to be nonnull, we never use it.
        // Note that `RTLD_DEFAULT` is often null, so
        // it really can't be used here.
        LibHandle::from(Some(&1))
    }
    fn load_sym(
        _: &LibHandle<'a, Self::Data>,
        fn_name: &CStr
    ) -> FnAddr {
        unsafe {
            dlsym(unix::RTLD_DEFAULT, fn_name.as_ptr())
        }
    }
}