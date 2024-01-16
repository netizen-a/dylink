#![cfg(target_os = "macos")]
use dylink::*;

#[test]
fn test_sym_hdr() {
	let lib = Library::open("libSystem.dylib").unwrap();
	let sym = lib.symbol("malloc").unwrap();
	let base = Symbol::image(sym);
	assert!(base.is_some())
}

#[test]
fn test_path() {
	let lib = Library::open("libSystem.dylib").unwrap();
	let path = lib.to_image().unwrap().path();
	assert!(path.is_ok())
}
