// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
#![allow(clippy::upper_case_acronyms)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

// setting stuff up right now. I know stuff isn't being used.
#![allow(dead_code)]

use std::ffi;
pub use std::os::windows::raw::HANDLE;

pub type HMODULE = HANDLE;
pub type PCWSTR = *const u16;
pub type PCSTR = *const ffi::c_char;
pub type PWSTR = *mut u16;
pub type BOOL = i32;
pub type DWORD = u32;
pub type WORD = u16;
pub type ULONGLONG = u64;
pub type BYTE = u8;

pub const IMAGE_NUMBEROF_DIRECTORY_ENTRIES: usize = 16;

#[repr(C)]
pub struct MODULEINFO {
	pub lpBaseOfDll: *mut ffi::c_void,
	pub SizeOfImage: DWORD,
	pub EntryPoint: *mut ffi::c_void,
}

#[repr(C)]
pub struct IMAGE_DATA_DIRECTORY {
	pub VirtualAddress: DWORD,
	pub Size: DWORD,
}

#[repr(C)]
pub struct IMAGE_OPTIONAL_HEADER32 {
	pub Magic: WORD,
	pub MajorLinkerVersion: BYTE,
	pub MinorLinkerVersion: BYTE,
	pub SizeOfCode: DWORD,
	pub SizeOfInitializedData: DWORD,
	pub SizeOfUninitializedData: DWORD,
	pub AddressOfEntryPoint: DWORD,
	pub BaseOfCode: DWORD,
	pub BaseOfData: DWORD,
	pub ImageBase: DWORD,
	pub SectionAlignment: DWORD,
	pub FileAlignment: DWORD,
	pub MajorOperatingSystemVersion: WORD,
	pub MinorOperatingSystemVersion: WORD,
	pub MajorImageVersion: WORD,
	pub MinorImageVersion: WORD,
	pub MajorSubsystemVersion: WORD,
	pub MinorSubsystemVersion: WORD,
	pub Win32VersionValue: DWORD,
	pub SizeOfImage: DWORD,
	pub SizeOfHeaders: DWORD,
	pub CheckSum: DWORD,
	pub Subsystem: WORD ,
	pub DllCharacteristics: WORD,
	pub SizeOfStackReserve: DWORD,
	pub SizeOfStackCommit: DWORD,
	pub SizeOfHeapReserve: DWORD,
	pub SizeOfHeapCommit: DWORD,
	pub LoaderFlags: DWORD,
	pub NumberOfRvaAndSizes: DWORD,
	pub DataDirectory: [IMAGE_DATA_DIRECTORY; IMAGE_NUMBEROF_DIRECTORY_ENTRIES],
}

#[repr(C)]
pub struct IMAGE_OPTIONAL_HEADER64 {
	pub Magic: WORD,
	pub MajorLinkerVersion: BYTE,
	pub MinorLinkerVersion: BYTE,
	pub SizeOfCode: DWORD,
	pub SizeOfInitializedData: DWORD,
	pub SizeOfUninitializedData: DWORD,
	pub AddressOfEntryPoint: DWORD,
	pub BaseOfCode: DWORD,
	pub ImageBase: ULONGLONG,
	pub SectionAlignment: DWORD,
	pub FileAlignment: DWORD,
	pub MajorOperatingSystemVersion: WORD,
	pub MinorOperatingSystemVersion: WORD,
	pub MajorImageVersion: WORD,
	pub MinorImageVersion: WORD,
	pub MajorSubsystemVersion: WORD,
	pub MinorSubsystemVersion: WORD,
	pub Win32VersionValue: DWORD,
	pub SizeOfImage: DWORD,
	pub SizeOfHeaders: DWORD,
	pub CheckSum: DWORD,
	pub Subsystem: WORD,
	pub DllCharacteristics: WORD,
	pub SizeOfStackReserve: ULONGLONG,
	pub SizeOfStackCommit: ULONGLONG,
	pub SizeOfHeapReserve: ULONGLONG,
	pub SizeOfHeapCommit: ULONGLONG,
	pub LoaderFlags: DWORD,
	pub NumberOfRvaAndSizes: DWORD,
	pub DataDirectory: [IMAGE_DATA_DIRECTORY; IMAGE_NUMBEROF_DIRECTORY_ENTRIES],
}

#[repr(C)]
pub struct IMAGE_FILE_HEADER {
	pub Machine: WORD,
	pub NumberOfSections: WORD,
	pub TimeDateStamp: DWORD,
	pub PointerToSymbolTable: DWORD,
	pub NumberOfSymbols: DWORD,
	pub SizeOfOptionalHeader: WORD,
	pub Characteristics: WORD,
}

#[repr(C)]
pub struct IMAGE_NT_HEADERS32 {
	pub Signature: DWORD,
	pub FileHeader: IMAGE_FILE_HEADER,
	pub OptionalHeader: IMAGE_OPTIONAL_HEADER32,
}

type IMAGE_OPTIONAL_HEADER = crate::sealed::Opaque;

#[repr(C)]
pub struct IMAGE_NT_HEADERS64 {
	pub Signature: DWORD,
	pub FileHeader: IMAGE_FILE_HEADER,
	pub OptionalHeader: IMAGE_OPTIONAL_HEADER64,
}

// This is suppose to be a DST, but limitations make it a ZST.
// size_of should not be used on this type.
#[repr(C)]
pub struct IMAGE_NT_HEADERS {
	pub Signature: DWORD,
	pub FileHeader: IMAGE_FILE_HEADER,
	OptionalHeader: IMAGE_OPTIONAL_HEADER,
}

extern "system" {
	pub fn LoadLibraryExW(lplibfilename: PCWSTR, hfile: HANDLE, dwflags: u32) -> HMODULE;
	pub fn GetModuleHandleExW(dwflags: u32, lpmodulename: PCWSTR, phmodule: *mut HMODULE) -> BOOL;
	pub fn GetProcAddress(handle: HMODULE, symbol: PCSTR) -> *const ffi::c_void;
	pub fn FreeLibrary(hlibmodule: *mut ffi::c_void) -> ffi::c_int;
	pub fn GetModuleFileNameW(hmodule: HMODULE, lpfilename: PWSTR, nsize: DWORD) -> DWORD;
	pub fn GetCurrentProcess() -> HANDLE;
	#[link_name = "K32EnumProcessModulesEx"]
	pub fn EnumProcessModulesEx(
		hprocess: HANDLE,
		lphmodule: *mut HMODULE,
		cb: DWORD,
		lpcbneeded: *mut DWORD,
		dwfilterflag: DWORD,
	) -> BOOL;
	#[link_name = "K32GetModuleInformation"]
	pub fn GetModuleInformation(
		hprocess: HANDLE,
		hmodule: HMODULE,
		lpmodinfo: *mut MODULEINFO,
		cb: DWORD,
	) -> BOOL;
}

// this should be replaced with a const function later.
#[link(name = "Dbghelp")]
extern "system" {
	pub fn ImageNtHeader(
		Base: *mut ffi::c_void
	) -> *mut IMAGE_NT_HEADERS;
}

pub const GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT: DWORD = 0x00000002u32;
pub const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS: DWORD = 0x00000004u32;

pub const LIST_MODULES_ALL: DWORD = 0x03;
