#![cfg(target_os = "macos")]

use dylink::*;

#[test]
fn test_sym_addr() {
	let lib = Library::open("libSystem.dylib").unwrap();
	let sym = lib.symbol("malloc").unwrap();
	let base = sym.base_addr().unwrap();
	println!("base address = {:p}", base);
}

#[test]
fn test_path() {
	let lib = Library::open("libSystem.dylib").unwrap();
	let path = lib.path().unwrap();
	println!("path = {}", path.display());
}

#[test]
fn test_metadata() {
	let lib = Library::open("libSystem.dylib").unwrap();
	let metadata = lib.metadata();
	println!("metadata = {:?}", metadata);
}

// test to see if there are race conditions when getting a path.
#[test]
fn test_path_soundness() {
	let mut vlib = vec![];
	for _ in 0..300 {
		vlib.push(Library::open("libSystem.dylib").unwrap())
	}
	let t = std::thread::spawn( || {
		let mut other_vlib = vec![];
		for _ in 0..300 {
			other_vlib.push(Library::open("libSystem.dylib").unwrap())
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