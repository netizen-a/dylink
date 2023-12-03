// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
#![allow(non_camel_case_types)]

use std::ffi;

type cpu_type_t = ffi::c_int;
type cpu_subtype_t = ffi::c_int;

// 32-bit header
#[repr(C)]
pub struct mach_header {
	pub magic: u32,
	pub cputype: cpu_type_t,
	pub cpusubtype: cpu_subtype_t,
	pub filetype: u32,
	pub ncmds: u32,
	pub sizeofcmds: u32,
	pub flags: u32,
}

// 64-bit header
#[repr(C)]
pub struct mach_header_64 {
	pub magic: u32,
	pub cputype: cpu_type_t,
	pub cpusubtype: cpu_subtype_t,
	pub filetype: u32,
	pub ncmds: u32,
	pub sizeofcmds: u32,
	pub flags: u32,
	pub reserved: u32,
}

#[cfg(target_os = "macos")]
pub type PfnImageCallback = extern "C" fn(mh: *const mach_header, vmaddr_slide: isize);

#[repr(C)]
pub struct Dl_info {
	pub dli_fname: *const ffi::c_char,
	pub dli_fbase: *mut ffi::c_void,
	pub dli_sname: *const ffi::c_char,
	pub dli_saddr: *mut ffi::c_void,
}

pub const RTLD_LOCAL: ffi::c_int = 0;
pub const RTLD_NOW: ffi::c_int = 0x2;
#[cfg(target_os = "macos")]
pub const RTLD_NOLOAD: ffi::c_int = 0x4;
#[cfg(target_env = "gnu")]
pub const RTLD_DI_LINKMAP: ffi::c_int = 2;
#[cfg(target_env = "gnu")]
pub type ElfW_Addr = usize;
#[cfg(target_env = "gnu")]
pub type Elf64_Xword = u64;

pub type ElfW_Half = u16;
pub type ElfW_Word = u32;

pub type Elf32_Off = u32;
pub type Elf64_Off = u64;

pub type Elf32_Addr = u32;

pub type Elf64_Addr = u64;

pub const ELFCLASS32: u8 = 1;
pub const ELFCLASS64: u8 = 2;

#[repr(C)]
pub struct Elf32_Ehdr {
	pub e_ident: [ffi::c_uchar; 16],
	pub e_type: ElfW_Half,
	pub e_machine: ElfW_Half,
	pub e_version: ElfW_Word,
	pub e_entry: Elf32_Addr,
	pub e_phoff: Elf32_Off,
	pub e_shoff: Elf32_Off,
	pub e_flags: ElfW_Word,
	pub e_ehsize: ElfW_Half,
	pub e_phentsize: ElfW_Half,
	pub e_phnum: ElfW_Half,
	pub e_shentsize: ElfW_Half,
	pub e_shnum: ElfW_Half,
	pub e_shstrndx: ElfW_Half,
}

#[repr(C)]
pub struct Elf64_Ehdr {
	pub e_ident: [ffi::c_uchar; 16],
	pub e_type: ElfW_Half,
	pub e_machine: ElfW_Half,
	pub e_version: ElfW_Word,
	pub e_entry: Elf64_Addr,
	pub e_phoff: Elf64_Off,
	pub e_shoff: Elf64_Off,
	pub e_flags: ElfW_Word,
	pub e_ehsize: ElfW_Half,
	pub e_phentsize: ElfW_Half,
	pub e_phnum: ElfW_Half,
	pub e_shentsize: ElfW_Half,
	pub e_shnum: ElfW_Half,
	pub e_shstrndx: ElfW_Half,
}

#[cfg(all(target_env = "gnu", target_pointer_width = "32"))]
#[repr(C)]
pub struct Elf32_Phdr {
	pub p_type: ElfW_Word,
	pub p_offset: Elf32_Off,
	pub p_vaddr: Elf32_Addr,
	pub p_paddr: Elf32_Addr,
	pub p_filesz: ElfW_Word,
	pub p_memsz: ElfW_Word,
	pub p_flags: ElfW_Word,
	pub p_align: ElfW_Word,
}

#[cfg(all(target_env = "gnu", target_pointer_width = "64"))]
#[repr(C)]
pub struct Elf64_Phdr {
	pub p_type: ElfW_Word,
	pub p_flags: ElfW_Word,
	pub p_offset: Elf64_Off,
	pub p_vaddr: Elf64_Addr,
	pub p_paddr: Elf64_Addr,
	pub p_filesz: Elf64_Xword,
	pub p_memsz: Elf64_Xword,
	pub p_align: Elf64_Xword,
}

#[cfg(target_os = "linux")]
#[repr(C)]
pub struct dl_phdr_info {
	pub dlpi_addr: ElfW_Addr,
	pub dlpi_name: *const ffi::c_char,
	#[cfg(target_pointer_width = "64")]
	pub dlpi_phdr: *const Elf64_Phdr,
	#[cfg(target_pointer_width = "32")]
	pub dlpi_phdr: *const Elf32_Phdr,
	pub dlpi_phnum: ElfW_Half,
}

extern "C" {
	pub fn dlopen(filename: *const ffi::c_char, flag: ffi::c_int) -> *mut ffi::c_void;
	pub fn dlerror() -> *const ffi::c_char;
	pub fn dlsym(handle: *mut ffi::c_void, symbol: *const ffi::c_char) -> *const ffi::c_void;
	pub fn dlclose(hlibmodule: *mut ffi::c_void) -> ffi::c_int;
	#[cfg(not(target_os = "aix"))]
	pub fn dladdr(addr: *const ffi::c_void, info: *mut Dl_info) -> ffi::c_int;
	#[cfg(target_env = "gnu")]
	pub fn dlinfo(
		handle: *mut ffi::c_void,
		request: ffi::c_int,
		info: *mut ffi::c_void,
	) -> ffi::c_int;
}

#[cfg(target_os = "linux")]
pub type DlIteratePhdrCallback = unsafe extern "C" fn(
	info: *mut dl_phdr_info,
	size: usize,
	data: *mut ffi::c_void,
) -> ffi::c_int;

#[cfg(target_os = "linux")]
extern "C" {
	pub fn dl_iterate_phdr(callback: DlIteratePhdrCallback, data: *mut ffi::c_void) -> ffi::c_int;
}

#[cfg(target_os = "macos")]
extern "C" {
	pub fn _dyld_get_image_name(image_index: u32) -> *const ffi::c_char;
	pub fn _dyld_register_func_for_add_image(func: PfnImageCallback);
	pub fn _dyld_register_func_for_remove_image(func: PfnImageCallback);
	// returns base address
	pub fn _dyld_get_image_header(image_index: u32) -> *const mach_header;
}

#[cfg(target_env = "gnu")]
#[repr(C)]
pub struct ElfW_Dyn {
	d_tag: usize,
	d_un: usize,
}

#[cfg(target_env = "gnu")]
#[repr(C)]
pub struct link_map {
	pub l_addr: usize,
	pub l_name: *mut ffi::c_char,
	pub l_ld: *mut ElfW_Dyn,
	pub l_next: *mut link_map,
	pub l_prev: *mut link_map,
	_marker: std::marker::PhantomPinned,
}
