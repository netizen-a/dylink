use dylink::*;

#[cfg(windows)]
static KERNEL32: sync::LibLock = sync::LibLock::new(&["Kernel32.dll"]);

#[cfg(target_os = "linux")]
static LIB_X11: sync::LibLock = sync::LibLock::new(&["libX11.so.6"]);

#[cfg(windows)]
#[test]
fn test_win32_kernel32() {
	#[dylink(library = KERNEL32)]
	extern "stdcall" {
		fn SetLastError(_: u32);
	}

	// macro output: function
	#[dylink(library = KERNEL32)]
	extern "system" fn GetLastError() -> u32;

	unsafe {
		SetLastError(53);
		assert_eq!(GetLastError(), 53);
	}
}

#[cfg(windows)]
#[test]
fn test_win32_impl() {
	#[repr(transparent)]
	struct Foo(u32);

	impl Foo {
		#[dylink(library = KERNEL32, link_name = "SetLastError")]
		extern "stdcall" fn set_last_error(self: Foo);

		#[dylink(library = KERNEL32)]
		extern "system" fn GetLastError() -> Foo;
	}

	let foo = Foo(23);
	unsafe {
		foo.set_last_error();
		assert!(Foo::GetLastError().0 == 23)
	}
}

#[test]
fn test_metadata() {
	let lib = lib!["libX11.so.6", "Kernel32.dll", "libSystem.dylib"].unwrap();
	let path = lib.path().unwrap();
	let metadata = lib.metadata();
	println!("path = {}", path.display());
	println!("metadata = {:?}", metadata);
	lib.close().unwrap();
}

#[test]
fn test_this_path() {
	let lib = Library::this().unwrap();
	let path = lib.path().unwrap();
	let metadata = lib.metadata();
	println!("path = {}", path.display());
	println!("metadata = {:?}", metadata);
	lib.close().unwrap();
}

#[cfg(any(windows, target_os="linux"))]
#[test]
fn test_sym_addr() {
	let lib = lib!["libX11.so.6", "Kernel32.dll"].unwrap();
	let sym = if cfg!(target_os="linux") {
		lib.symbol("XOpenDisplay").unwrap()
	} else if cfg!(windows) {
		lib.symbol("SetLastError").unwrap()
	} else {
		unreachable!()
	};
	let base = sym.base_addr().unwrap();
	println!("base address = {:p}", base);
}

#[cfg(any(feature = "unstable", docsrs))]
#[cfg(not(any(windows, target_os = "aix")))]
#[test]
fn test_unix_sym_info() {
	use dylink::os::unix::SymExt;
	let this = Library::this().unwrap();
	let symbol = this.symbol("atoi").unwrap();
	let info = symbol.info();
	println!("{:?}", info);
}

#[cfg(target_os = "linux")]
#[test]
fn test_linux_x11() {
	use std::ffi::{c_char, c_void, CStr};

	#[repr(transparent)]
	struct Display(*const c_void);

	#[dylink(library = LIB_X11)]
	extern "C" {
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

#[cfg(target_os = "linux")]
#[test]
fn test_atoi_linux() {
	use std::ffi::{c_char, c_int};
	static THIS: sync::LibLock = sync::LibLock::this();
	#[dylink(library=THIS)]
	extern "C-unwind" {
		fn atoi(s: *const c_char) -> c_int;
	}

	let five = unsafe { atoi(b"5\0".as_ptr().cast()) };
	assert_eq!(five, 5);
}
