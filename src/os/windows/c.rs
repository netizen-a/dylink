#![allow(clippy::upper_case_acronyms)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi;
pub use std::os::windows::raw::HANDLE;

pub type HMODULE = HANDLE;
pub type PCWSTR = *const u16;
pub type PCSTR = *const ffi::c_char;
pub type PWSTR = *mut u16;
pub type BOOL = i32;
pub type DWORD = u32;
pub type ULONG = ffi::c_ulong;
pub type ULONG64 = u64;
pub type DWORD64 = u64;
pub type PSYM_ENUMMODULES_CALLBACKW64 =
	unsafe extern "system-unwind" fn(PCWSTR, DWORD64, *mut ffi::c_void) -> BOOL;
pub type PSYM_ENUMERATESYMBOLS_CALLBACKW =
	unsafe extern "system-unwind" fn(*mut SYMBOL_INFOW, ULONG, *mut ffi::c_void) -> BOOL;

pub const MAX_SYM_NAME: u32 = 2000u32;

// this structure is variable in length
#[derive(Debug)]
#[repr(C)]
pub struct SYMBOL_INFOW {
	pub sizeofstruct: ULONG,
	pub typeindex: ULONG,
	pub reserved: [ULONG64; 2],
	pub index: ULONG,
	pub size: ULONG,
	pub modbase: ULONG64,
	pub flags: ULONG,
	pub value: ULONG64,
	pub address: ULONG64,
	pub register: ULONG,
	pub scope: ULONG,
	pub tag: ULONG,
	pub namelen: ULONG,
	pub maxnamelen: ULONG,
	pub name: [u16; 1],
}

extern "system" {
	pub fn LoadLibraryExW(lplibfilename: PCWSTR, hfile: HANDLE, dwflags: u32) -> HMODULE;
	pub fn GetModuleHandleExW(dwflags: u32, lpmodulename: PCWSTR, phmodule: *mut HMODULE) -> BOOL;
	pub fn GetProcAddress(handle: HMODULE, symbol: PCSTR) -> *const ffi::c_void;
	pub fn FreeLibrary(hlibmodule: *mut ffi::c_void) -> ffi::c_int;
	pub fn GetModuleFileNameW(hmodule: HMODULE, lpfilename: PWSTR, nsize: DWORD) -> DWORD;
	pub fn FreeLibraryAndExitThread(hLibModule: HMODULE, dwExitCode: DWORD) -> !;
	pub fn GetCurrentProcess() -> HANDLE;
	pub fn wcslen(buf: *const u16) -> usize;
}

#[link(name = "Dbghelp")]
extern "system" {
	pub fn SymInitializeW(hprocess: HANDLE, usersearchpath: PCWSTR, finvadeprocess: BOOL) -> BOOL;
	pub fn SymCleanup(process: HANDLE) -> BOOL;
	pub fn SymFromAddrW(
		hprocess: HANDLE,
		address: DWORD64,
		displacement: *mut DWORD64,
		symbol: *mut SYMBOL_INFOW,
	) -> BOOL;
	pub fn SymSetOptions(symoptions: DWORD) -> DWORD;
	pub fn SymGetOptions() -> DWORD;
	pub fn SymEnumerateModulesW64(
		hprocess: HANDLE,
		enummodulescallback: PSYM_ENUMMODULES_CALLBACKW64,
		usercontext: *mut ffi::c_void,
	) -> BOOL;
	pub fn SymEnumSymbolsExW(
		hProcess: HANDLE,
		BaseOfDll: ULONG64,
		Mask: PCWSTR,
		EnumSymbolsCallback: PSYM_ENUMERATESYMBOLS_CALLBACKW,
		UserContext: *mut ffi::c_void,
		Options: DWORD,
	) -> BOOL;
}

pub const GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT: DWORD = 0x00000002u32;
pub const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS: DWORD = 0x00000004u32;

pub const SYMOPT_UNDNAME: DWORD = 0x00000002;
pub const SYMOPT_DEFERRED_LOADS: DWORD = 0x00000004;
