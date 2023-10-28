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
