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
fn test_sym_addr() -> std::io::Result<()> {
	let lib = Library::open("libX11.so.6")?;
	let sym = lib.symbol("XOpenDisplay")?;
	let base = sym.base_address()?;
	println!("base address = {:p}", base);
	Ok(())
}

#[test]
fn test_path() -> std::io::Result<()> {
	let lib = Library::open("libX11.so.6")?;
	let path = lib.path()?;
	println!("path = {}", path.display());
	Ok(())
}

#[test]
fn test_metadata() {
	let lib = Library::open("libX11.so.6").unwrap();
	let metadata = lib.metadata();
	println!("metadata = {:?}", metadata);
}

// test to see if there are race conditions when getting a path.
#[test]
fn test_path_soundness() {
	let mut vlib = vec![];
	for _ in 0..300 {
		vlib.push(Library::open("libX11.so.6").unwrap())
	}
	let t = std::thread::spawn(|| {
		let mut other_vlib = vec![];
		for _ in 0..300 {
			other_vlib.push(Library::open("libX11.so.6").unwrap())
		}
		for lib in other_vlib.drain(0..) {
			let _ = lib.path().unwrap();
		}
	});
	for lib in vlib.drain(0..) {
		let _ = lib.path().unwrap();
	}
	t.join().unwrap();
}
