#![cfg(windows)]

use dylink::*;

#[cfg(windows)]
static KERNEL32: sync::LibLock = sync::LibLock::new(&["Kernel32.dll"]);

#[cfg(windows)]
#[test]
fn test_macro() {
	#[dylink(library = KERNEL32)]
	extern "system-unwind" {
		fn SetLastError(_: u32);
	}

	// macro output: function
	#[dylink(library = KERNEL32)]
	extern "system-unwind" fn GetLastError() -> u32;

	unsafe {
		SetLastError(53);
		assert_eq!(GetLastError(), 53);
	}
}

#[cfg(windows)]
#[test]
fn test_macro_impl() {
	#[repr(transparent)]
	struct Foo(u32);

	impl Foo {
		#[dylink(library = KERNEL32, link_name = "SetLastError")]
		extern "system-unwind" fn set_last_error(self: Foo);

		#[dylink(library = KERNEL32)]
		extern "system-unwind" fn GetLastError() -> Foo;
	}

	let foo = Foo(23);
	unsafe {
		foo.set_last_error();
		assert!(Foo::GetLastError().0 == 23)
	}
}

#[test]
fn test_sym_addr() {
	let lib = Library::open("Kernel32.dll").unwrap();
	let sym = lib.symbol("SetLastError").unwrap();
	let base = sym.base_addr().unwrap();
	println!("base address = {:p}", base);
}

#[test]
fn test_path() {
	let lib = Library::open("Kernel32.dll").unwrap();
	let path = lib.path().unwrap();
	println!("path = {}", path.display());
}

#[test]
fn test_metadata() {
	let lib = Library::open("Kernel32.dll").unwrap();
	let metadata = lib.metadata();
	println!("metadata = {:?}", metadata);
}

// test to see if there are race conditions when getting a path.
#[test]
fn test_path_soundness() {
	let mut vlib = vec![];
	for _ in 0..300 {
		vlib.push(Library::open("Kernel32.dll").unwrap())
	}
	let t = std::thread::spawn(|| {
		let mut other_vlib = vec![];
		for _ in 0..300 {
			other_vlib.push(Library::open("Kernel32.dll").unwrap())
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

#[test]
fn test_objects(){
	let _objs = iter::Objects::now().unwrap();
}