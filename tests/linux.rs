// SPDX-FileCopyrightText: 2022-2026 Jonathan A. Thomason <contact@jonathan-thomason.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(target_os = "linux")]
use dylink::*;

static LIB_X11: sync::LibLock = sync::LibLock::new(&["libX11.so.6"]);

#[test]
fn test_linux_x11() {
	use std::ffi::{
		CStr,
		c_char,
		c_void,
	};

	type Display = c_void;

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
	use std::ffi::{
		c_char,
		c_int,
	};
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
	let base = Symbol::image(sym);
	assert!(base.is_some())
}

#[test]
fn test_path() {
	let lib = Library::open("libX11.so.6").unwrap();
	let path = lib.to_image().unwrap().path();
	assert!(path.is_ok())
}

/// Test that to_bytes returns at least the full image size for each loaded image.
/// This is a portable test that works across all platforms.
#[test]
fn to_bytes_returns_full_image_size() {
	let images = img::Images::now().expect("Should get images");
	for weak in images {
		let lib = weak.upgrade().expect("Should upgrade to strong reference");
		let image = lib.to_image().expect("Should get image from library");
		let bytes = image.to_bytes().expect("Should get bytes from image");

		if let Ok(path) = image.path() {
			if let Ok(metadata) = std::fs::metadata(&path) {
				let file_size = metadata.len() as usize;
				assert!(
					bytes.len() >= file_size,
					"to_bytes returned {} bytes for '{}', expected at least {file_size} bytes (file size)",
					bytes.len(),
					path.display()
				);
			}
		}
	}
}
