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

	let list = unsafe {[CStr::from_bytes_with_nul_unchecked(b"Kernel32.dll\0")]};
	let lazyfn: LazyFn::<PfnTy> = LazyFn::<PfnTy>::new(
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
	use std::ffi::{c_char, CStr};
	use dylink::*;

	extern "C" {
		fn dlerror() -> *const c_char;
	}

	#[dylink(name = "libX11.so.6", strip=true)]
	extern "C" {
		fn foo();
	}

	unsafe {
		match foo.try_link() {
			Ok(func) => func(),
			Err(DylinkError::FnNotFound(_)) => {
				println!("library was found and successfully linked. dummy function `foo` was not loaded as expected")
			}
			Err(_) => {
				let c_str = CStr::from_ptr(dlerror());
				panic!("{}", c_str.to_string_lossy());
			}
		}		
	}
}
