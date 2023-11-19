#![cfg(target_os = "macos")]

use dylink::*;

#[test]
fn test_sym_addr() {
	let lib = Library::open("libSystem.dylib").unwrap();
	let sym = lib.symbol("malloc").unwrap();
	let base = sym.base_address();
	assert!(!base.is_null())
}

#[test]
fn test_path() {
	let lib = Library::open("libSystem.dylib").unwrap();
	let path = lib.path();
	assert!(path.is_ok())
}
