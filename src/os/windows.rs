use std::os::windows::prelude::*;
use std::path::PathBuf;
use std::{ffi, io, mem, path, ptr};

use crate::img;
use crate::weak;
use crate::{Library, Symbol};

mod c;

fn to_wide(path: &ffi::OsStr) -> Vec<u16> {
	path.encode_wide().chain(std::iter::once(0u16)).collect()
}

#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct InnerLibrary(pub std::ptr::NonNull<ffi::c_void>);

impl InnerLibrary {
	pub unsafe fn open(path: &ffi::OsStr) -> io::Result<Self> {
		let wide_str: Vec<u16> = to_wide(path);
		let handle = c::LoadLibraryExW(wide_str.as_ptr(), ptr::null_mut(), 0);
		ptr::NonNull::new(handle)
			.ok_or_else(io::Error::last_os_error)
			.map(Self)
	}

	pub unsafe fn this() -> io::Result<Self> {
		let mut handle: *mut ffi::c_void = ptr::null_mut();
		c::GetModuleHandleExW(0, ptr::null(), &mut handle);
		ptr::NonNull::new(handle)
			.ok_or_else(io::Error::last_os_error)
			.map(Self)
	}

	#[inline]
	pub unsafe fn raw_symbol(&self, name: &ffi::CStr) -> *const Symbol {
		c::GetProcAddress(self.0.as_ptr(), name.as_ptr()).cast()
	}

	pub unsafe fn symbol<'a>(&self, name: &str) -> io::Result<*const Symbol> {
		let c_str = ffi::CString::new(name).unwrap();
		let addr = self.raw_symbol(&c_str);
		if addr.is_null() {
			Err(io::Error::last_os_error())
		} else {
			Ok(addr)
		}
	}

	pub(crate) unsafe fn path(&self) -> io::Result<path::PathBuf> {
		const MAX_PATH: usize = 260;
		const ERROR_INSUFFICIENT_BUFFER: i32 = 0x7A;

		let mut file_name = vec![0u16; MAX_PATH];
		loop {
			let _ = c::GetModuleFileNameW(
				self.0.as_ptr(),
				file_name.as_mut_ptr(),
				file_name.len() as c::DWORD,
			);
			let last_error = io::Error::last_os_error();
			match last_error.raw_os_error().unwrap_unchecked() {
				0 => {
					// The function succeeded.
					// Truncate the vector to remove unused zero bytes.
					if let Some(new_len) = file_name.iter().rposition(|&a| a != 0) {
						file_name.truncate(new_len + 1)
					}
					let os_str = ffi::OsString::from_wide(&file_name);
					break Ok(os_str.into());
				}
				ERROR_INSUFFICIENT_BUFFER => {
					// The buffer is too small; double its size.
					file_name.resize(file_name.len() * 2, 0)
				}
				_ => {
					// An unexpected error occurred; return an error.
					return Err(last_error);
				}
			}
		}
	}
	pub(crate) unsafe fn try_clone(&self) -> io::Result<Self> {
		let mut new_handle = ptr::null_mut();
		let _ = c::GetModuleHandleExW(
			c::GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
			self.0.as_ptr().cast(),
			&mut new_handle,
		);
		ptr::NonNull::new(new_handle)
			.ok_or_else(io::Error::last_os_error)
			.map(Self)
	}
	pub(crate) unsafe fn from_ptr(addr: *mut img::Image) -> Option<Self> {
		if let Some(addr) = ptr::NonNull::new(addr.cast::<ffi::c_void>()) {
			let new_lib = InnerLibrary(addr);
			new_lib.try_clone().ok()
		} else {
			None
		}
	}

	#[inline]
	pub(crate) unsafe fn to_ptr(&self) -> *const img::Image {
		self.0.as_ptr().cast()
	}
}

impl Drop for InnerLibrary {
	fn drop(&mut self) {
		unsafe {
			c::FreeLibrary(self.0.as_ptr());
		}
	}
}

impl AsHandle for Library {
	fn as_handle(&self) -> BorrowedHandle<'_> {
		unsafe { BorrowedHandle::borrow_raw(self as *const _ as *mut _) }
	}
}

impl AsRawHandle for Library {
	fn as_raw_handle(&self) -> RawHandle {
		self as *const _ as *mut _
	}
}

pub(crate) unsafe fn base_addr(symbol: *const Symbol) -> *mut img::Image {
	let mut handle = ptr::null_mut();
	let _ = c::GetModuleHandleExW(
		c::GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT | c::GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
		symbol.cast(),
		&mut handle,
	);
	handle.cast()
}

pub(crate) unsafe fn load_objects() -> io::Result<Vec<weak::Weak>> {
	const INITIAL_SIZE: usize = 1000;
	let process_handle = c::GetCurrentProcess();
	let mut module_handles = vec![ptr::null_mut::<img::Image>(); INITIAL_SIZE];
	let mut len_needed: u32 = 0;
	let mut prev_size = INITIAL_SIZE;

	loop {
		let cb = (module_handles.len() * mem::size_of::<c::HANDLE>()) as u32;
		let result = c::EnumProcessModulesEx(
			process_handle,
			module_handles.as_mut_ptr().cast(),
			cb,
			&mut len_needed,
			c::LIST_MODULES_ALL,
		);
		if result == 0 {
			return Err(io::Error::last_os_error());
		}
		len_needed /= mem::size_of::<c::HANDLE>() as u32;
		if len_needed as usize > module_handles.len() {
			// We can't trust the next iteration to be bigger, so fill with null
			module_handles.fill(ptr::null_mut());
			// make the new size sufficiently bigger, and always grow instead of shrink.
			let new_size: usize = (prev_size).max(len_needed as usize + 30);
			prev_size = new_size;
			module_handles.resize(new_size, ptr::null_mut());
		} else {
			// success, so truncate to the appropriate size
			if let Some(new_len) = module_handles.iter().rposition(|a| !a.is_null()) {
				module_handles.truncate(new_len)
			}
			let module_handles = module_handles
				.into_iter()
				.map(|base_addr| {
					let base_nonnull = ptr::NonNull::new_unchecked(base_addr.cast());
					let hmodule = mem::ManuallyDrop::new(InnerLibrary(base_nonnull));
					weak::Weak {
						base_addr,
						path_name: hmodule.path().ok(),
					}
				})
				.collect::<Vec<weak::Weak>>();
			// box and return the slice
			return Ok(module_handles);
		}
	}
}

pub(crate) unsafe fn hdr_size(hdr: *const img::Image) -> io::Result<usize> {
	// checks if it's a PE header (fast)
	let pe_hdr = c::ImageNtHeader(hdr as *const _ as *mut _);
	// if it's PE we can skip all sys calls and return the size immediately.
	if !pe_hdr.is_null() {
		let pe_hdr32 = pe_hdr as *mut c::IMAGE_NT_HEADERS32;
		return Ok((*pe_hdr32).optionalheader.sizeofimage as usize);
	}

	let hprocess = c::GetCurrentProcess();
	let hmodule = hdr as *mut ffi::c_void;
	let mut lpmodinfo = mem::MaybeUninit::zeroed();
	let cb = mem::size_of::<c::MODULEINFO>();
	let result = c::GetModuleInformation(hprocess, hmodule, lpmodinfo.as_mut_ptr(), cb as u32);
	if result != 0 {
		Ok(lpmodinfo.assume_init().sizeofimage as usize)
	} else {
		Err(io::Error::last_os_error())
	}
}

pub(crate) unsafe fn hdr_path(hdr: *const img::Image) -> io::Result<PathBuf> {
	let Some(nonnull_hdr) = ptr::NonNull::new(hdr as *mut _) else {
		return Err(io::Error::new(io::ErrorKind::Other, "invalid header"));
	};
	let lib = mem::ManuallyDrop::new(InnerLibrary(nonnull_hdr));
	lib.path()
}

mod tests {
	#[test]
	fn test_size() {
		use super::*;
		let imgs = crate::img::Images::now().unwrap();
		for weak in imgs {
			println!("{}", weak.path().unwrap().display());
			let img = weak.base_addr;
			let hdr = unsafe { c::ImageNtHeader(weak.base_addr.cast_mut().cast()) };
			assert!(!hdr.is_null(), "{:?}", std::io::Error::last_os_error());
			let hdr32 = hdr as *mut c::IMAGE_NT_HEADERS32;
			let hdr_len = unsafe { hdr_size(img).unwrap() };
			let img_len = unsafe { (*hdr32).optionalheader.sizeofimage };
			assert!(img_len == hdr_len as u32, "{img_len} == {hdr_len}");
		}
	}
}
