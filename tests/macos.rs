#![cfg(target_os = "macos")]

use dylink::*;

#[test]
fn test_sym_hdr() {
	let lib = Library::open("libSystem.dylib").unwrap();
	let sym = lib.symbol("malloc").unwrap();
	let base = sym.header();
	assert!(base.is_some())
}

#[test]
fn test_path() {
	let lib = Library::open("libSystem.dylib").unwrap();
	let path = lib.path();
	assert!(path.is_ok())
}
