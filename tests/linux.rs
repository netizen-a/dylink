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

/// Computes the byte span of an ELF file by finding the maximum offset+size
/// across all section headers, which represents the actual loaded image extent.
fn compute_elf_span(path: &std::path::Path) -> Option<usize> {
	let data = std::fs::read(path).ok()?;

	let elf_class = data[4];
	match elf_class {
		1 => elf_span_32(&data),
		2 => elf_span_64(&data),
		_ => None,
	}
}

fn elf_span_32(data: &[u8]) -> Option<usize> {
	if data.len() < 52 {
		return None;
	}

	let e_shoff = u32::from_le_bytes([data[32], data[33], data[34], data[35]]) as usize;
	let e_shentsize = u16::from_le_bytes([data[48], data[49]]) as usize;
	let e_shnum = u16::from_le_bytes([data[46], data[47]]) as usize;

	let mut max_end = 0usize;
	for i in 0..e_shnum {
		let off = e_shoff + (i * e_shentsize);
		if off + 40 > data.len() {
			break;
		}
		let sh_offset = u32::from_le_bytes([
			data[off + 24],
			data[off + 25],
			data[off + 26],
			data[off + 27],
		]) as usize;
		let sh_size = u32::from_le_bytes([
			data[off + 28],
			data[off + 29],
			data[off + 30],
			data[off + 31],
		]) as usize;
		max_end = max_end.max(sh_offset + sh_size);
	}

	Some(max_end as usize)
}

fn elf_span_64(data: &[u8]) -> Option<usize> {
	if data.len() < 64 {
		return None;
	}

	let e_shoff = u64::from_le_bytes([
		data[40], data[41], data[42], data[43], data[44], data[45], data[46], data[47],
	]) as usize;
	let e_shentsize = u16::from_le_bytes([data[48], data[49]]) as usize;
	let e_shnum = u16::from_le_bytes([data[46], data[47]]) as usize;

	let mut max_end = 0usize;
	for i in 0..e_shnum {
		let off = e_shoff + (i * e_shentsize);
		if off + 64 > data.len() {
			break;
		}
		let sh_offset = u64::from_le_bytes([
			data[off + 24],
			data[off + 25],
			data[off + 26],
			data[off + 27],
			data[off + 28],
			data[off + 29],
			data[off + 30],
			data[off + 31],
		]);
		let sh_size = u64::from_le_bytes([
			data[off + 32],
			data[off + 33],
			data[off + 34],
			data[off + 35],
			data[off + 36],
			data[off + 37],
			data[off + 38],
			data[off + 39],
		]);
		max_end = max_end.max((sh_offset + sh_size) as usize);
	}

	Some(max_end)
}

/// Test that to_bytes returns at least the full image size for each loaded image.
#[test]
fn to_bytes_returns_full_image_size() {
	let images = img::Images::now().expect("Should get images");
	for weak in images {
		let lib = weak.upgrade().expect("Should upgrade to strong reference");
		let image = lib.to_image().expect("Should get image from library");
		let bytes = image.to_bytes().expect("Should get bytes from image");

		if let Ok(path) = image.path() {
			const ELF_MAGIC: &[u8] = &[0x7f, b'E', b'L', b'F'];
			if !path.exists() || !bytes.starts_with(ELF_MAGIC) {
				continue;
			}
			let Some(elf_span) = compute_elf_span(&path) else {
				panic!("Failed to compute ELF span for '{}'", path.display());
			};
			assert!(
				bytes.len() >= elf_span,
				"to_bytes returned {} bytes for ELF '{}', expected at least {elf_span} bytes (ELF section span)",
				bytes.len(),
				path.display()
			);
		}
		let _ = lib.close();
	}
}
