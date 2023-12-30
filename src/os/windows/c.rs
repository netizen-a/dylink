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
	mem, ptr,
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

#[cfg(target_pointer_width = "64")]
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
	fn GetSystemInfo(lpsysteminfo: *mut SYSTEM_INFO);
}

#[derive(Clone, Copy)]
#[repr(C)]
struct SYSTEM_INFO_0_0 {
	pub wprocessorarchitecture: WORD,
	pub wreserved: WORD,
}

#[repr(C)]
union SYSTEM_INFO_0 {
	pub dwoemid: DWORD,
	pub anonymous: SYSTEM_INFO_0_0,
}

#[repr(C)]
struct SYSTEM_INFO {
	anonymous: SYSTEM_INFO_0,
	dwpagesize: DWORD,
	lpminimumapplicationaddress: *mut ffi::c_void,
	lpmaximumapplicationaddress: *mut ffi::c_void,
	dwactiveprocessormask: usize,
	dwnumberofprocessors: DWORD,
	dwprocessortype: DWORD,
	dwallocationgranularity: DWORD,
	wprocessorlevel: WORD,
	wprocessorrevision: WORD,
}

#[repr(C)]
pub struct IMAGE_DOS_HEADER {
	pub e_magic: WORD,
	pub e_cblp: WORD,
	pub e_cp: WORD,
	pub e_crlc: WORD,
	pub e_cparhdr: WORD,
	pub e_minalloc: WORD,
	pub e_maxalloc: WORD,
	pub e_ss: WORD,
	pub e_sp: WORD,
	pub e_csum: WORD,
	pub e_ip: WORD,
	pub e_cs: WORD,
	pub e_lfarlc: WORD,
	pub e_ovno: WORD,
	pub e_res: [WORD; 4],
	pub e_oemid: WORD,
	pub e_oeminfo: WORD,
	pub e_res2: [WORD; 10],
	pub e_lfanew: i32,
}

const IMAGE_DOS_SIGNATURE: u16 = 0x5A4D;
const IMAGE_DOS_SIGNATURE2: u16 = 0x4D5A;
const IMAGE_NT_SIGNATURE: u32 = u32::from_le_bytes([b'P', b'E', 0, 0]);

// thread-safe version of win32's ImageNtHeader.
pub unsafe fn ImageNtHeader(base: *mut IMAGE_DOS_HEADER) -> *mut IMAGE_NT_HEADERS {
	use std::sync::OnceLock;
	static DOS_SIZE: OnceLock<u32> = OnceLock::new();
	if base.is_null() {
		return ptr::null_mut();
	}
	let dos_hdr = &mut *base;

	// check if the DOS siginature
	if dos_hdr.e_magic != IMAGE_DOS_SIGNATURE && (*base).e_magic != IMAGE_DOS_SIGNATURE2 {
		return ptr::null_mut();
	}

	// cache and get the dos size
	let dos_size = DOS_SIZE.get_or_init(|| {
		let mut sys_info = mem::MaybeUninit::<SYSTEM_INFO>::zeroed();
		GetSystemInfo(sys_info.as_mut_ptr());
		let sys_info = sys_info.assume_init();
		let page_size = sys_info.dwpagesize;
		dos_hdr.e_cp as u32 * page_size - dos_hdr.e_cblp as u32
	});

	// check if the PE header offset is within bounds
	if *dos_size + 4 < dos_hdr.e_lfanew as u32 {
		return ptr::null_mut();
	}

	// calculate the new offset and return a pointer to the NT header
	let pe_hdr = (base as *mut u8).offset(dos_hdr.e_lfanew as isize) as *mut IMAGE_NT_HEADERS;

	// we need to check if it's really the PE header. If the magic matches then return a pointer to the header.
	if (*pe_hdr).signature == IMAGE_NT_SIGNATURE {
		pe_hdr
	} else {
		ptr::null_mut()
	}
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
