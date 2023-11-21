#![cfg(target_os = "linux")]
use dylink::*;

static LIB_X11: sync::LibLock = sync::LibLock::new(&["libX11.so.6"]);

#[test]
fn test_linux_x11() {
	use std::ffi::{c_char, c_void, CStr};

	#[repr(transparent)]
	struct Display(*const c_void);

	#[dylink(library = LIB_X11)]
	extern "C-unwind" {
		fn XOpenDisplay(display_name: *const c_char) -> *mut Display;
		fn XCloseDisplay(display: *mut Display);
	}

	unsafe {
		let display_name = CStr::from_bytes_until_nul(b"foo\0").unwrap();
		let disp = XOpenDisplay(display_name.as_ptr());
		if !disp.is_null() {
			println!("display created successfully.\nnow destroying...");
			XCloseDisplay(disp);
		}
	}
}

#[test]
fn test_atoi_linux() {
	use std::ffi::{c_char, c_int};
	static THIS: sync::LibLock = sync::LibLock::new(&[]);
	#[dylink(library=THIS)]
	extern "C-unwind" {
		fn atoi(s: *const c_char) -> c_int;
	}

	let five = unsafe { atoi(b"5\0".as_ptr().cast()) };
	assert_eq!(five, 5);
}

#[test]
fn test_sym_hdr() {
	let lib = Library::open("libX11.so.6").unwrap();
	let sym = lib.symbol("XOpenDisplay").unwrap();
	let base = sym.header();
	assert!(base.is_some())
}

#[test]
fn test_path() {
	let lib = Library::open("libX11.so.6").unwrap();
	let path = lib.path();
	assert!(path.is_ok())
}


#[test]
fn test_hdr_convert() {
	let images = img::Images::now().unwrap();
	for img in images {
		let maybe_hdr = unsafe { img.to_ptr().as_ref() };
		let Some(hdr) = maybe_hdr else {
			continue;
		};
		let bytes = hdr.to_bytes().unwrap();
		if bytes[4] == libc::ELFCLASS32 {
			let hdr: &libc::Elf32_Ehdr = hdr.try_into().unwrap();
			assert_eq!(hdr.e_version, libc::EV_CURRENT);
		} else if bytes[4] == libc::ELFCLASS64 {
			let hdr: &libc::Elf64_Ehdr = hdr.try_into().unwrap();
			assert_eq!(hdr.e_version, libc::EV_CURRENT);
		} else {
			panic!("unknown header class")
		}
	}
}