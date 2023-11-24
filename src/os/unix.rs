// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
#![allow(clippy::let_unit_value)]

use crate::sealed::Sealed;
use crate::{img, weak, Symbol};
use std::marker::PhantomData;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::{ffi, io, mem, path::PathBuf, ptr};

#[cfg(target_os = "macos")]
use std::sync::{
	atomic::{AtomicU32, Ordering},
	Once,
};

#[cfg(not(any(target_os = "linux", target_os = "macos", target_env = "gnu")))]
use std::sync::{LockResult, Mutex, MutexGuard};

mod c;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_env = "gnu")))]
#[inline]
fn dylib_guard<'a>() -> LockResult<MutexGuard<'a, ()>> {
	static LOCK: Mutex<()> = Mutex::new(());
	LOCK.lock()
}

#[cfg(any(target_os = "linux", target_os = "macos", target_env = "gnu"))]
#[inline(always)]
fn dylib_guard() {}

unsafe fn c_dlerror() -> Option<ffi::CString> {
	let raw = c::dlerror();
	if raw.is_null() {
		None
	} else {
		Some(ffi::CStr::from_ptr(raw).to_owned())
	}
}

#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct InnerLibrary(ptr::NonNull<ffi::c_void>);

impl InnerLibrary {
	pub unsafe fn open(path: &ffi::OsStr) -> io::Result<Self> {
		let _lock = dylib_guard();
		let c_str = ffi::CString::new(path.as_bytes())?;
		let handle: *mut ffi::c_void = c::dlopen(c_str.as_ptr(), c::RTLD_NOW | c::RTLD_LOCAL);
		if let Some(ret) = ptr::NonNull::new(handle) {
			Ok(Self(ret))
		} else {
			let err = c_dlerror().unwrap();
			Err(io::Error::new(io::ErrorKind::Other, err.to_string_lossy()))
		}
	}
	pub unsafe fn this() -> io::Result<Self> {
		let _lock = dylib_guard();
		let handle: *mut ffi::c_void = c::dlopen(ptr::null(), c::RTLD_NOW | c::RTLD_LOCAL);
		if let Some(ret) = ptr::NonNull::new(handle) {
			Ok(Self(ret))
		} else {
			let err = c_dlerror().unwrap();
			Err(io::Error::new(io::ErrorKind::Other, err.to_string_lossy()))
		}
	}

	#[inline]
	pub unsafe fn c_symbol(&self, name: &ffi::CStr) -> *const ffi::c_void {
		c::dlsym(self.0.as_ptr(), name.as_ptr())
	}

	pub unsafe fn symbol<'a>(&self, name: &str) -> io::Result<Symbol<'a>> {
		let _lock = dylib_guard();
		let c_str = ffi::CString::new(name).unwrap();

		let _ = c_dlerror(); // clear existing errors
		let handle: *mut ffi::c_void = self.c_symbol(&c_str).cast_mut();

		if let Some(err) = c_dlerror() {
			Err(io::Error::new(io::ErrorKind::Other, err.to_string_lossy()))
		} else {
			Ok(Symbol(handle, PhantomData))
		}
	}
	pub unsafe fn path(&self) -> io::Result<PathBuf> {
		match Self::this() {
			Ok(this_handle)
				if (cfg!(target_os = "macos")
					&& (this_handle.0.as_ptr() as isize & (-4))
						== (self.0.as_ptr() as isize & (-4)))
					|| this_handle.0 == self.0 =>
			{
				std::env::current_exe()
			}
			_ => {
				#[cfg(target_env = "gnu")]
				{
					if let Some(path) = self.get_link_map_path() {
						Ok(path)
					} else {
						Err(io::Error::new(
							io::ErrorKind::NotFound,
							"Library path not found",
						))
					}
				}
				#[cfg(target_os = "macos")]
				{
					self.get_macos_image_path()
				}
				#[cfg(not(any(target_env = "gnu", target_os = "macos")))]
				{
					// Handle other platforms or configurations
					Err(io::Error::new(io::ErrorKind::Other, "Unsupported platform"))
				}
			}
		}
	}
	pub(crate) unsafe fn try_clone(&self) -> io::Result<Self> {
		let this = Self::this()?;
		if this.0 == self.0 {
			Ok(this)
		} else {
			std::mem::drop(this);
			let path = self.path()?;
			Self::open(path.as_os_str())
		}
	}

	// returns null if handle is invalid
	#[cfg(target_env = "gnu")]
	pub(crate) unsafe fn to_ptr(&self) -> *const img::Header {
		let mut map_ptr = ptr::null_mut::<c::link_map>();
		if c::dlinfo(
			self.0.as_ptr(),
			c::RTLD_DI_LINKMAP,
			&mut map_ptr as *mut _ as *mut _,
		) == 0
		{
			(*map_ptr).l_addr as *const img::Header
		} else {
			ptr::null()
		}
	}

	// returns null if handle is invalid
	#[cfg(target_os = "macos")]
	pub(crate) unsafe fn to_ptr(&self) -> *const img::Header {
		let handle = self.0;
		let mut result = ptr::null();
		let _ = get_image_count().fetch_update(Ordering::SeqCst, Ordering::SeqCst, |image_index| {
			for image_index in (0..image_index).rev() {
				let image_name = c::_dyld_get_image_name(image_index);
				let active_handle =
					c::dlopen(image_name, c::RTLD_NOW | c::RTLD_LOCAL | c::RTLD_NOLOAD);
				if !active_handle.is_null() {
					let _ = c::dlclose(active_handle);
				}
				if (handle.as_ptr() as isize & (-4)) == (active_handle as isize & (-4)) {
					result = c::_dyld_get_image_header(image_index) as *const img::Header;
					break;
				}
			}
			Some(image_index)
		});
		result
	}
	pub(crate) unsafe fn from_ptr(addr: *const img::Header) -> Option<Self> {
		let mut info = mem::MaybeUninit::zeroed();
		if c::dladdr(addr.cast(), info.as_mut_ptr()) != 0 {
			let info = info.assume_init();
			let handle = c::dlopen(info.dli_fname, c::RTLD_NOW | c::RTLD_LOCAL);
			ptr::NonNull::new(handle).map(Self)
		} else {
			None
		}
	}
	#[cfg(target_env = "gnu")]
	unsafe fn get_link_map_path(&self) -> Option<PathBuf> {
		let handle = self.0;
		let mut map_ptr = ptr::null_mut::<c::link_map>();
		if c::dlinfo(
			handle.as_ptr(),
			c::RTLD_DI_LINKMAP,
			&mut map_ptr as *mut _ as *mut _,
		) == 0
		{
			let path = ffi::CStr::from_ptr((*map_ptr).l_name);
			let path = ffi::OsStr::from_bytes(path.to_bytes());
			if !path.is_empty() {
				Some(path.into())
			} else {
				None
			}
		} else {
			None
		}
	}
	#[cfg(target_os = "macos")]
	unsafe fn get_macos_image_path(&self) -> io::Result<PathBuf> {
		let handle = self.0;
		let mut result = Err(io::Error::new(io::ErrorKind::NotFound, "Path not found"));
		let _ = get_image_count().fetch_update(Ordering::SeqCst, Ordering::SeqCst, |image_index| {
			for image_index in (0..image_index).rev() {
				let image_name = c::_dyld_get_image_name(image_index);
				let active_handle =
					c::dlopen(image_name, c::RTLD_NOW | c::RTLD_LOCAL | c::RTLD_NOLOAD);
				if !active_handle.is_null() {
					let _ = c::dlclose(active_handle);
				}
				if (handle.as_ptr() as isize & (-4)) == (active_handle as isize & (-4)) {
					let path = ffi::CStr::from_ptr(image_name);
					let path = ffi::OsStr::from_bytes(path.to_bytes());
					result = Ok(path.into());
					break;
				}
			}
			Some(image_index)
		});
		result
	}
}
impl Drop for InnerLibrary {
	fn drop(&mut self) {
		unsafe { c::dlclose(self.0.as_ptr()) };
	}
}

#[cfg(target_os = "macos")]
fn get_image_count() -> &'static AtomicU32 {
	static IMAGE_COUNT: AtomicU32 = AtomicU32::new(0);
	static START: Once = Once::new();
	extern "C" fn increment_count(_: *const c::mach_header, _: isize) {
		IMAGE_COUNT.fetch_add(1, Ordering::SeqCst);
	}
	extern "C" fn decrement_count(_: *const c::mach_header, _: isize) {
		IMAGE_COUNT.fetch_sub(1, Ordering::SeqCst);
	}
	START.call_once(|| unsafe {
		c::_dyld_register_func_for_add_image(increment_count);
		c::_dyld_register_func_for_remove_image(decrement_count);
	});

	&IMAGE_COUNT
}

pub(crate) unsafe fn base_addr(symbol: *mut std::ffi::c_void) -> *mut img::Header {
	let mut info = mem::MaybeUninit::<c::Dl_info>::zeroed();
	if c::dladdr(symbol, info.as_mut_ptr()) != 0 {
		let info = info.assume_init();
		info.dli_fbase.cast()
	} else {
		ptr::null_mut()
	}
}

#[derive(Debug)]
pub struct DlInfo {
	pub dli_fname: ffi::CString,
	pub dli_fbase: *mut img::Header,
	pub dli_sname: ffi::CString,
	pub dli_saddr: *mut ffi::c_void,
}

pub trait SymExt: Sealed {
	fn info(&self) -> io::Result<DlInfo>;
}

impl SymExt for Symbol<'_> {
	#[doc(alias = "dladdr")]
	fn info(&self) -> io::Result<DlInfo> {
		let mut info = mem::MaybeUninit::<c::Dl_info>::zeroed();
		unsafe {
			if c::dladdr(self.0 as *const _, info.as_mut_ptr()) != 0 {
				let info = info.assume_init();
				Ok(DlInfo {
					dli_fname: ffi::CStr::from_ptr(info.dli_fname).to_owned(),
					dli_fbase: info.dli_fbase.cast(),
					dli_sname: ffi::CStr::from_ptr(info.dli_sname).to_owned(),
					dli_saddr: info.dli_saddr,
				})
			} else {
				// dlerror isn't available for dlinfo, so I can only provide a general error message here
				Err(io::Error::new(
					io::ErrorKind::Other,
					"Failed to retrieve symbol information",
				))
			}
		}
	}
}

#[cfg(target_env = "gnu")]
unsafe fn iter_phdr<F>(mut f: F) -> ffi::c_int
where
	F: FnMut(*mut c::dl_phdr_info, usize) -> ffi::c_int,
{
	unsafe extern "C" fn callback<F>(
		info: *mut c::dl_phdr_info,
		size: usize,
		data: *mut ffi::c_void,
	) -> ffi::c_int
	where
		F: FnMut(*mut c::dl_phdr_info, usize) -> ffi::c_int,
	{
		let f = data as *mut F;
		(*f)(info, size)
	}
	c::dl_iterate_phdr(callback::<F>, &mut f as *mut _ as *mut _)
}

#[cfg(target_env = "gnu")]
pub(crate) unsafe fn load_objects() -> io::Result<Vec<weak::Weak>> {
	let mut data = Vec::new();
	let _ = iter_phdr(|info, _| {
		let path_name = if (*info).dlpi_name.is_null() {
			None
		} else if (*info).dlpi_name.read() == 0i8 {
			std::env::current_exe().ok()
		} else {
			let path = ffi::CStr::from_ptr((*info).dlpi_name);
			let path = ffi::OsStr::from_bytes(path.to_bytes());
			Some(PathBuf::from(path))
		};
		let weak_ptr = weak::Weak {
			base_addr: (*info).dlpi_addr as *mut img::Header,
			path_name,
		};
		data.push(weak_ptr);
		0
	});
	Ok(data)
}

#[cfg(target_os = "macos")]
pub(crate) unsafe fn load_objects() -> io::Result<Vec<weak::Weak>> {
	let mut data = Vec::new();
	let _ = get_image_count().fetch_update(Ordering::SeqCst, Ordering::SeqCst, |image_index| {
		data.clear();
		for image_index in 0..image_index {
			let path = ffi::CStr::from_ptr(c::_dyld_get_image_name(image_index));
			let path = ffi::OsStr::from_bytes(path.to_bytes());
			let weak_ptr = weak::Weak {
				base_addr: c::_dyld_get_image_header(image_index) as *const img::Header,
				path_name: Some(PathBuf::from(path)),
			};
			data.push(weak_ptr);
		}
		Some(image_index)
	});
	Ok(data)
}

pub(crate) unsafe fn hdr_size(hdr: *const img::Header) -> io::Result<usize> {
	#[cfg(target_os = "macos")]
	const MH_MAGIC: &[u8] = &0xfeedface_u32.to_le_bytes();
	#[cfg(target_os = "macos")]
	const MH_MAGIC_64: &[u8] = &0xfeedfacf_u32.to_le_bytes();
	#[cfg(not(target_os = "macos"))]
	const ELF_MAGIC: &[u8] = &[0x7f, b'E', b'L', b'F'];
	let magic: &[u8] = hdr.as_ref().unwrap_unchecked().magic();
	match magic {
		#[cfg(target_os = "macos")]
		MH_MAGIC => {
			let hdr = hdr as *const c::mach_header;
			Ok(mem::size_of::<c::mach_header>() + (*hdr).sizeofcmds as usize)
		}
		#[cfg(target_os = "macos")]
		MH_MAGIC_64 => {
			let hdr = hdr as *const c::mach_header_64;
			Ok(mem::size_of::<c::mach_header_64>() + (*hdr).sizeofcmds as usize)
		}
		#[cfg(not(target_os = "macos"))]
		ELF_MAGIC => {
			let data: *const u8 = hdr as *const u8;
			match *data.offset(4) {
				c::ELFCLASS32 => {
					let hdr = hdr as *const c::Elf32_Ehdr;
					Ok((*hdr).e_ehsize as usize)
				}
				c::ELFCLASS64 => {
					let hdr = hdr as *const c::Elf64_Ehdr;
					Ok((*hdr).e_ehsize as usize)
				}
				_ => Err(io::Error::new(
					io::ErrorKind::InvalidData,
					"invalid ELF file",
				)),
			}
		}
		_ => Err(io::Error::new(
			io::ErrorKind::Other,
			"unknown header detected",
		)),
	}
}
