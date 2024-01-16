#![cfg(windows)]
use dylink::*;

static KERNEL32: sync::LibLock = sync::LibLock::new(&["Kernel32.dll"]);

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
fn test_sym_img() {
	let lib = Library::open("Kernel32.dll").unwrap();
	let sym = lib.symbol("SetLastError").unwrap();
	let base = Symbol::image(sym);
	assert!(base.is_some())
}

#[test]
fn test_path() {
	let lib = Library::open("Kernel32.dll").unwrap();
	let path = lib.to_image().unwrap().path();
	assert!(path.is_ok())
}
