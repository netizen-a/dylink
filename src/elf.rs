#![allow(non_camel_case_types)]
pub const EI_NIDENT: usize = 16;
pub const EI_MAG0: u8 = 0x7f;
pub const EI_MAG1: u8 = b'E';
pub const EI_MAG2: u8 = b'L';
pub const EI_MAG3: u8 = b'F';

pub const ELFCLASSNONE: u8 = 0;
pub const ELFCLASS32: u8 = 1;
pub const ELFCLASS64: u8 = 2;

pub const ELFDATANONE: u8 = 0;
pub const ELFDATA2LSB: u8 = 1;
pub const ELFDATA2MSB: u8 = 2;

/// Invalid version
pub const EV_NONE: u8 = 0;
/// Current version
pub const EV_CURRENT: u8 = 1;


pub const ELFOSABI_SYSV: u8 = 0;
pub const ELFOSABI_HPUX: u8 = 1;
pub const ELFOSABI_NETBSD: u8 = 2;
pub const ELFOSABI_LINUX: u8 = 3;
pub const ELFOSABI_SOLARIS: u8 = 6;
pub const ELFOSABI_IRIX: u8 = 8;
pub const ELFOSABI_FREEBSD: u8 = 9;
pub const ELFOSABI_TRU64: u8 = 10;
pub const ELFOSABI_ARM: u8 = 97;
pub const ELFOSABI_STANDALONE: u8 = 255;


pub const EI_ABIVERSION: u8 = 0;
pub const EI_PAD: u8 = 0;


pub trait Arch {
    type UType;
    type IType;
}

impl Arch for u32 {
    type UType = u32;
    type IType = i32;
}
impl Arch for u64 {
    type UType = u64;
    type IType = i64;
}
impl Arch for i32 {
    type UType = u32;
    type IType = i32;
}
impl Arch for i64 {
    type UType = u64;
    type IType = i64;
}

#[repr(C)]
pub struct ElfN_Ehdr<T: Arch> {
	pub e_ident: [u8; EI_NIDENT],
	pub e_type: u16,
	pub e_machine: u16,
	pub e_version: u32,
	pub e_entry: ElfN_Addr<T>,
	pub e_phoff: ElfN_Off<T>,
	pub e_shoff: ElfN_Off<T>,
	pub e_flags: u32,
	pub e_ehsize: u16,
	pub e_phentsize: u16,
	pub e_phnum: u16,
	pub e_shentsize: u16,
	pub e_shnum: u16,
	pub e_shstrndx: u16,
}
#[cfg_attr(not(doc), repr(transparent))]
#[derive(Debug, Clone, Copy)]
pub struct ElfN_Addr<T: Arch>(T::UType);
#[cfg_attr(not(doc), repr(transparent))]
#[derive(Debug, Clone, Copy)]
pub struct ElfN_Off<T: Arch>(T::UType);

pub type Elf32_Ehdr = ElfN_Ehdr<u32>;
pub type Elf64_Ehdr = ElfN_Ehdr<u64>;
pub type Elf32_Addr = ElfN_Addr<u32>;
pub type Elf64_Addr = ElfN_Addr<u64>;
pub type Elf32_Off = ElfN_Off<u32>;
pub type Elf64_Off = ElfN_Off<u64>;

pub type Elf_Byte = u8;
pub type ElfN_Section = u16;
pub type ElfN_Versym = u16;
pub type ElfN_Half = u16;
pub type ElfN_Sword = i32;
pub type ElfN_Word = u32;
pub type ElfN_Sxword = i64;
pub type ElfN_Xword = u64;

#[repr(C)]
pub struct Elf32_Phdr {
    pub p_type: u32,
    pub p_offset: Elf32_Off,
    pub p_vaddr: Elf32_Addr,
    pub p_paddr: Elf32_Addr,
    pub p_filesz: u32,
    pub p_memsz:  u32,
    pub p_flags:  u32,
    pub p_align:  u32,
}

#[repr(C)]
pub struct Elf64_Phdr {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: Elf64_Off,
    pub p_vaddr: Elf64_Addr,
    pub p_paddr: Elf64_Addr,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

#[repr(C)]
pub struct ElfN_Shdr<T: Arch> {
    pub sh_name: u32,
    pub sh_type: u32,
    pub sh_flags: T,
    pub sh_addr: ElfN_Addr<T>,
    pub sh_offset: ElfN_Off<T>,
    pub sh_size: T::UType,
    pub sh_link: u32,
    pub sh_info: u32,
    pub sh_addralign: T::UType,
    pub sh_entsize: T::UType,
}

pub type Elf32_Shdr = ElfN_Shdr<u32>;
pub type Elf64_Shdr = ElfN_Shdr<u64>;

pub struct Elf32_Sym {
    pub st_name: u32,
    pub st_value: Elf32_Addr,
    pub st_size: u32,
    pub st_info: u8,
    pub st_other: u8,
    pub st_shndx: u16,
}

pub struct Elf64_Sym {
    pub st_name: u32,
    pub st_info: u8,
    pub st_other: u8,
    pub st_shndx: u16,
    pub st_value: Elf64_Addr,
    pub st_size: u64,
}

pub struct ElfN_Rel<T:Arch> {
    pub r_offset: ElfN_Addr<T>,
    pub r_info: T,
}

pub type Elf32_Rel = ElfN_Rel<u32>;
pub type Elf64_Rel = ElfN_Rel<u64>;

pub struct ElfN_Rela<T: Arch> {
    pub r_offset: ElfN_Addr<T>,
    pub r_info: T,
    pub r_addend: T::IType,
}

pub type Elf32_Rela = ElfN_Rela<u32>;
pub type Elf64_Rela = ElfN_Rela<u64>;

pub struct Elf32_Dyn {
    pub d_tag: ElfN_Sword,
    pub d_un: u32,
}

pub struct Elf64_Dyn {
    pub d_tag: ElfN_Sxword,
    pub d_un: u32,
}

pub struct ElfN_Nhdr {
    n_namesz: ElfN_Word,
    n_descsz: ElfN_Word,
    n_type: ElfN_Word,
}

pub type Elf32_Nhdr = ElfN_Nhdr;
pub type Elf64_Nhdr = ElfN_Nhdr;
