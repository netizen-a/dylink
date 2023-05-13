// apparently lacking unix tests...
#[cfg(windows)]
use dylink::*;

#[cfg(windows)]
#[test]
fn test_win32_kernel32() {
	// macro output: static variable
	#[dylink(name = "Kernel32.dll", strip = true)]
	extern "stdcall" {
		fn SetLastError(_: u32);
	}

	// macro output: function
	#[dylink(name = "Kernel32.dll", strip = false)]
	extern "C" {
		fn GetLastError() -> u32;
	}

	unsafe {
		// static variable has crappy documentation, but can be use for library induction.
		match SetLastError.try_link() {
			Ok(f) => f(53),
			Err(e) => panic!("{}", e),
		}
		assert_eq!(GetLastError(), 53);
	}
}

// tbh I don't know why this test passes.
#[cfg(windows)]
#[test]
fn test_win32_lifetimes() {
	use std::ffi::CStr;
	use std::ops::Deref;

	extern "stdcall" fn foo() -> u32 {
		0
	}
	type PfnTy = extern "stdcall" fn() -> u32;

	let list = unsafe { [CStr::from_bytes_with_nul_unchecked(b"Kernel32.dll\0")] };
	let lazyfn: LazyFn<PfnTy> = LazyFn::<PfnTy>::new(
		&(foo as PfnTy),
		unsafe { CStr::from_bytes_with_nul_unchecked(b"SetLastError\0") },
		dylink::LinkType::System(&list),
	);
	// `deref` isn't suppose to be used this way, but if
	// it is used, this test will check if it's valid.
	let old_ref = lazyfn.deref();
	let new_addr = lazyfn.try_link().unwrap();

	assert_eq!(*old_ref, foo as PfnTy);
	assert_ne!(*new_addr, foo as PfnTy);
	assert_ne!(lazyfn.deref(), old_ref);
}

#[cfg(windows)]
#[test]
fn test_fn_not_found() {
	#[dylink(name = "Kernel32.dll", strip = true)]
	extern "C" {
		fn foo();
	}

	match foo.try_link() {
		Ok(_) => unreachable!(),
		Err(err) => {
			println!("{err}")
		}
	}
}

#[cfg(target_os = "linux")]
#[test]
fn test_linux_x11() {
	use dylink::*;
	use std::ffi::{c_char, CStr, c_void};

	#[repr(transparent)]
	struct Display(*const c_void);

	#[dylink(name = "libX11.so.6", strip = true)]
	extern "C" {
		fn XOpenDisplay(display_name: *const c_char) -> *mut Display;
		fn XCloseDisplay(display: *mut Display);
	}

	unsafe {
		match XOpenDisplay.try_link() {
			Ok(func) => {
				let display_name = CStr::from_bytes_until_nul(b"foo\0").unwrap();
				let disp = func(display_name.as_ptr());
				if !disp.is_null() {
					println!("display created successfully.\nnow destroying...");
					XCloseDisplay(disp);
				}
			}
			Err(e) => {
				panic!("{e}");
			}
		}
	}
}

#[test]
#[should_panic]
fn test_multiple_lib_panic() {
	use dylink::*;

	#[dylink(
		any(name = "test_lib0", name = "test_lib1", name = "test_lib2"),
		strip = true
	)]
	extern "C" {
		fn foo();
	}

	unsafe {
		match foo.try_link() {
			Ok(func) => func(),
			Err(e) => {
				panic!("{e}");
			}
		}
	}
}
