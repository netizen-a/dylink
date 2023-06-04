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
	extern "system" fn GetLastError() -> u32;

	unsafe {
		// static variable has crappy documentation, but can be use for library induction.
		match SetLastError.try_link() {
			Ok(f) => f(53),
			Err(e) => panic!("{}", e),
		}
		assert_eq!(GetLastError(), 53);
	}
}

#[cfg(windows)]
#[test]
fn test_win32_impl() {
	#[repr(transparent)]
	struct Foo(u32);

	// TODO: Self and self (by itself) are impossible to implement, so consider giving hard errors.
	impl Foo {
		#[dylink(name = "Kernel32.dll", link_name = "SetLastError")]
		extern "stdcall" fn set_last_error(self: Foo);

		#[dylink(name = "Kernel32.dll")]
		extern "system" fn GetLastError() -> Foo;
	}

	let foo = Foo(23);
	unsafe {
		foo.set_last_error();
		assert!(Foo::GetLastError().0 == 23)
	}
}

// This test works because the AtomicPtr is referencing external data when it's first initialized
// When it's initialized, a new reference to a different memory location is created.
#[cfg(windows)]
#[test]
fn test_win32_lifetimes() {
	use std::ffi::CStr;
	use dylink::link;
	use std::ops::Deref;

	extern "stdcall" fn foo() -> u32 {
		1234
	}
	type PfnTy = extern "stdcall" fn() -> u32;

	let list = unsafe { [CStr::from_bytes_with_nul_unchecked(b"Kernel32.dll\0")] };
	let lazyfn: LazyFn<PfnTy, link::System> = LazyFn::<PfnTy, link::System>::new(
		&(foo as PfnTy),
		unsafe { CStr::from_bytes_with_nul_unchecked(b"SetLastError\0") },
		dylink::LinkType::General(&list),
	);
	// `deref` isn't suppose to be used this way, but if
	// it is used, this test will check if it's valid.
	let old_ref = lazyfn.deref();
	let new_ref = lazyfn.try_link().unwrap();
	assert!(old_ref() == 1234);

	// This is verbose like this because GitHub Actions keeps giving me `error[E0369]`.
	assert!(*old_ref as isize == foo as PfnTy as isize);
	assert!(new_ref as isize != foo as PfnTy as isize);
	assert!(new_ref as isize != *old_ref as isize);
	assert!(lazyfn.deref() as *const PfnTy as isize != old_ref as *const PfnTy as isize);
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
	use std::ffi::{c_char, c_void, CStr};

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

#[cfg(unix)]
#[test]
fn test_unix_libc() {
	#[cfg_attr(target_os = "linux", dylink(name = "libc.so.6", strip = true))]
	#[cfg_attr(target_os = "macos", dylink(name = "libc.dylib", strip = true))]
	extern "C" {
		fn foo();
	}

	match foo.try_link() {
		Ok(_) => unreachable!(),
		Err(DylinkError::FnNotFound(err)) => {
			println!("{err}")
		}
		Err(DylinkError::LibNotLoaded(err)) => panic!("e0\n{err}"),
		Err(DylinkError::ListNotLoaded(err)) => panic!("e1\n{err}"),
		Err(_) => todo!(),
	}
}

#[cfg(windows)]
#[test]
fn test_kernel32_unload() {
	use dylink::*;
	use std::ffi::CStr;

	#[dylink(name = "Kernel32.dll")]
	extern "system" {
	    fn GetLastError() -> u32;
	}
	
	unsafe {
	   	let _ = GetLastError();
		let lib_name = CStr::from_bytes_with_nul(b"Kernel32.dll\0").unwrap();
		link::System::unload(lib_name).unwrap();
	}
}

#[cfg(windows)]
#[test]
fn test_custom_linker() {
	use dylink::{*, link::*};
	use std::ffi::*;
	struct MyLinker;
	struct MyData();
	unsafe impl Sync for MyData {}
	unsafe impl Send for MyData {}

	impl RTLinker for MyLinker {
	    type Data = Box<u32>;
	    fn load_lib(_: &CStr) -> LibHandle<'static, Self::Data> {
			LibHandle::from(None)
	    }
	    fn load_sym(
	        _: &LibHandle<'static, Self::Data>,
	        _: &CStr,
	    ) -> FnAddr {
			std::ptr::null()
	    }
	}

 	#[dylink(name = "my_lib.dll", linker=MyLinker)]
	extern "C" {
		fn foo() -> u32;
	}
}