#![allow(clippy::upper_case_acronyms)]

use std::ffi;
use std::os::windows::raw;

pub type HMODULE = raw::HANDLE;
pub type PCWSTR = *const u16;
pub type PCSTR = *const ffi::c_char;
pub type PWSTR = *mut u16;
pub type BOOL = i32;
pub type DWORD = u32;

extern "stdcall" {
	pub fn LoadLibraryExW(lplibfilename: PCWSTR, hfile: raw::HANDLE, dwflags: u32) -> HMODULE;
	pub fn GetModuleHandleExW(dwflags: u32, lpmodulename: PCWSTR, phmodule: *mut HMODULE) -> BOOL;
	pub fn GetProcAddress(handle: HMODULE, symbol: PCSTR) -> *const ffi::c_void;
	pub fn FreeLibrary(hlibmodule: *mut ffi::c_void) -> ffi::c_int;
	pub fn GetModuleFileNameW(hmodule: HMODULE, lpfilename: PWSTR, nsize: DWORD) -> DWORD;
	pub fn FreeLibraryAndExitThread(hLibModule: HMODULE, dwExitCode: DWORD) -> !;
}

pub const GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT: DWORD = 0x00000002u32;