#![allow(clippy::upper_case_acronyms)]
#![allow(non_snake_case)]

use std::ffi;
pub use std::os::windows::raw::HANDLE;

pub type HMODULE = HANDLE;
pub type PCWSTR = *const u16;
pub type PCSTR = *const ffi::c_char;
pub type PWSTR = *mut u16;
pub type BOOL = i32;
pub type DWORD = u32;
//pub type ULONG = ffi::c_ulong;
//pub type ULONG64 = u64;
//pub type CHAR = ffi::c_char;
//pub type DWORD64 = u64;
//// this structure is variable in length
//#[repr(C)]
//pub struct SYMBOL_INFO {
//	SizeOfStruct: ULONG,
//	TypeIndex: ULONG,
//	Reserved: [ULONG64; 2],
//	Index: ULONG,
//	Size: ULONG,
//	ModBase: ULONG64,
//	Flags: ULONG,
//	Value: ULONG64,
//	Address: ULONG64,
//	Register: ULONG,
//	Scope: ULONG,
//	Tag: ULONG,
//	NameLen: ULONG,
//	MaxNameLen: ULONG,
//	Name: [CHAR; 1],
//}


extern "stdcall" {
	pub fn LoadLibraryExW(lplibfilename: PCWSTR, hfile: HANDLE, dwflags: u32) -> HMODULE;
	pub fn GetModuleHandleExW(dwflags: u32, lpmodulename: PCWSTR, phmodule: *mut HMODULE) -> BOOL;
	pub fn GetProcAddress(handle: HMODULE, symbol: PCSTR) -> *const ffi::c_void;
	pub fn FreeLibrary(hlibmodule: *mut ffi::c_void) -> ffi::c_int;
	pub fn GetModuleFileNameW(hmodule: HMODULE, lpfilename: PWSTR, nsize: DWORD) -> DWORD;
	pub fn FreeLibraryAndExitThread(hLibModule: HMODULE, dwExitCode: DWORD) -> !;
	pub fn GetCurrentProcess() -> HANDLE;
}

#[link(name = "Dbghelp")]
extern "stdcall" {
	pub fn SymInitializeW(hprocess: HANDLE, usersearchpath: PCWSTR, finvadeprocess: BOOL) -> BOOL;
	pub fn SymCleanup(process: HANDLE) -> BOOL;
	//pub fn SymFromAddr(
	//	hprocess: HANDLE,
	//	address: DWORD64,
	//	displacement: *mut DWORD64,
	//	symbol: *mut SYMBOL_INFO,
	//) -> BOOL;
	//pub fn SymDeleteSymbol(
	//	hprocess: HANDLE,
	//	baseofdll: ULONG64,
	//	name: PCSTR,
	//	address: DWORD64,
	//	flags: DWORD,
	//) -> BOOL;
}

pub const GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT: DWORD = 0x00000002u32;
pub const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS: DWORD = 0x00000004u32;
