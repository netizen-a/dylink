// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
#![allow(clippy::upper_case_acronyms)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
// setting stuff up right now. I know stuff isn't being used.
#![allow(dead_code)]

pub use std::os::windows::raw::HANDLE;
use std::{
	ffi,
	marker::{PhantomData, PhantomPinned},
};

pub type HMODULE = HANDLE;
pub type PCWSTR = *const u16;
pub type PCSTR = *const ffi::c_char;
pub type PSTR = *mut ffi::c_char;
pub type PWSTR = *mut u16;
pub type BOOL = i32;
pub type DWORD = u32;
pub type WORD = u16;
pub type ULONGLONG = u64;
pub type BOOLEAN = u8;

pub const IMAGE_NUMBEROF_DIRECTORY_ENTRIES: usize = 16;

#[repr(C)]
pub struct MODULEINFO {
	pub lpbaseofdll: *mut ffi::c_void,
	pub sizeofimage: DWORD,
	pub entrypoint: *mut ffi::c_void,
}

#[repr(C)]
pub struct IMAGE_DATA_DIRECTORY {
	pub virtualaddress: DWORD,
	pub size: DWORD,
}

#[repr(C)]
pub struct IMAGE_OPTIONAL_HEADER32 {
	pub magic: WORD,
	pub majorlinkerversion: u8,
	pub minorlinkerversion: u8,
	pub sizeofcode: DWORD,
	pub SizeOfInitializedData: DWORD,
	pub SizeOfUninitializedData: DWORD,
	pub addressofentrypoint: DWORD,
	pub baseofcode: DWORD,
	pub baseofdata: DWORD,
	pub imagebase: DWORD,
	pub sectionalignment: DWORD,
	pub filealignment: DWORD,
	pub majoroperatingsystemversion: WORD,
	pub minoroperatingsystemversion: WORD,
	pub majorimageversion: WORD,
	pub minorimageversion: WORD,
	pub majorsubsystemversion: WORD,
	pub minorsubsystemversion: WORD,
	pub win32versionvalue: DWORD,
	pub sizeofimage: DWORD,
	pub sizeofheaders: DWORD,
	pub checksum: DWORD,
	pub subsystem: WORD,
	pub dllcharacteristics: WORD,
	pub sizeofstackreserve: DWORD,
	pub sizeofstackcommit: DWORD,
	pub sizeofheapreserve: DWORD,
	pub sizeofheapcommit: DWORD,
	pub loaderflags: DWORD,
	pub numberofrvaandsizes: DWORD,
	pub datadirectory: [IMAGE_DATA_DIRECTORY; IMAGE_NUMBEROF_DIRECTORY_ENTRIES],
}

#[repr(C)]
pub struct IMAGE_OPTIONAL_HEADER64 {
	pub magic: WORD,
	pub majorlinkerversion: u8,
	pub minorlinkerversion: u8,
	pub sizeofcode: DWORD,
	pub sizeofinitializeddata: DWORD,
	pub sizeofuninitializeddata: DWORD,
	pub addressofentrypoint: DWORD,
	pub baseofcode: DWORD,
	pub imagebase: ULONGLONG,
	pub sectionalignment: DWORD,
	pub filealignment: DWORD,
	pub majoroperatingsystemversion: WORD,
	pub minoroperatingsystemversion: WORD,
	pub majorimageversion: WORD,
	pub minorimageversion: WORD,
	pub majorsubsystemversion: WORD,
	pub minorsubsystemversion: WORD,
	pub win32versionvalue: DWORD,
	pub sizeofimage: DWORD,
	pub sizeofheaders: DWORD,
	pub checksum: DWORD,
	pub subsystem: WORD,
	pub dllcharacteristics: WORD,
	pub sizeofstackreserve: ULONGLONG,
	pub sizeofstackcommit: ULONGLONG,
	pub sizeofheapreserve: ULONGLONG,
	pub sizeofheapcommit: ULONGLONG,
	pub loaderflags: DWORD,
	pub numberofrvaandsizes: DWORD,
	pub datadirectory: [IMAGE_DATA_DIRECTORY; IMAGE_NUMBEROF_DIRECTORY_ENTRIES],
}

#[repr(C)]
pub struct IMAGE_FILE_HEADER {
	pub machine: WORD,
	pub numberofsections: WORD,
	pub timedatestamp: DWORD,
	pub pointertosymboltable: DWORD,
	pub numberofsymbols: DWORD,
	pub sizeofoptionalheader: WORD,
	pub characteristics: WORD,
}

#[repr(C)]
pub struct IMAGE_NT_HEADERS32 {
	pub signature: DWORD,
	pub fileheader: IMAGE_FILE_HEADER,
	pub optionalheader: IMAGE_OPTIONAL_HEADER32,
}

type IMAGE_OPTIONAL_HEADER = crate::sealed::Opaque;

#[repr(C)]
pub struct IMAGE_NT_HEADERS64 {
	pub signature: DWORD,
	pub fileheader: IMAGE_FILE_HEADER,
	pub optionalheader: IMAGE_OPTIONAL_HEADER64,
}

// This is suppose to be a DST, but limitations make it a ZST.
// size_of should not be used on this type.
#[repr(C)]
pub struct IMAGE_NT_HEADERS {
	pub signature: DWORD,
	pub fileheader: IMAGE_FILE_HEADER,
	_optionalheader: IMAGE_OPTIONAL_HEADER,
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
	pub fn MapAndLoad(
		imagename: PCSTR,
		dllpath: PCSTR,
		loadedimage: *mut LOADED_IMAGE,
		dotdll: BOOL,
		readonly: BOOL,
	) -> BOOL;
	pub fn UnMapAndLoad(loadedimage: *mut LOADED_IMAGE) -> BOOL;
}

// this should be replaced with a const function later.
#[link(name = "Dbghelp")]
extern "system" {
	pub fn ImageNtHeader(base: *mut ffi::c_void) -> *mut IMAGE_NT_HEADERS;
}

pub const GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT: DWORD = 0x00000002u32;
pub const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS: DWORD = 0x00000004u32;

pub const LIST_MODULES_ALL: DWORD = 0x03;
pub const IMAGE_SIZEOF_SHORT_NAME: usize = 8;

#[repr(C)]
pub union _Misc {
	pub physicaladdress: DWORD,
	pub virtualsize: DWORD,
}

#[repr(C)]
pub struct IMAGE_SECTION_HEADER {
	pub name: [u8; IMAGE_SIZEOF_SHORT_NAME],
	pub misc: _Misc,
	pub virtualaddress: DWORD,
	pub sizeofrawdata: DWORD,
	pub pointertorawdata: DWORD,
	pub pointertorelocations: DWORD,
	pub pointertolinenumbers: DWORD,
	pub numberofrelocations: WORD,
	pub numberoflinenumbers: WORD,
	pub characteristics: DWORD,
}

#[repr(C)]
pub struct LIST_ENTRY {
	pub flink: *mut LIST_ENTRY,
	pub blink: *mut LIST_ENTRY,
	_marker: PhantomData<PhantomPinned>,
}

#[repr(C)]
pub struct LOADED_IMAGE {
	pub modulename: PSTR,
	pub hfile: HANDLE,
	pub mappedaddress: *mut ffi::c_uchar,
	pub fileheader: *mut IMAGE_NT_HEADERS,
	pub lastrvasection: *mut IMAGE_SECTION_HEADER,
	pub numberofsections: ffi::c_ulong,
	pub sections: *mut IMAGE_SECTION_HEADER,
	pub characteristics: ffi::c_ulong,
	pub fsystemimage: BOOLEAN,
	pub fdosimage: BOOLEAN,
	pub freadonly: BOOLEAN,
	pub version: ffi::c_uchar,
	pub links: LIST_ENTRY,
	pub sizeofimage: ffi::c_ulong,
}
