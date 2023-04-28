#[cfg(windows)]
#[test]
fn test_win32_kernel32() {
	use dylink::dylink;
	use windows_sys::Win32::Foundation::{ERROR_INVALID_PRINTER_COMMAND, WIN32_ERROR};
	#[dylink(name = "Kernel32.dll")]
	extern "stdcall" {
		fn SetLastError(_: WIN32_ERROR);
		fn GetLastError() -> WIN32_ERROR;
	}
	unsafe {
		SetLastError(ERROR_INVALID_PRINTER_COMMAND);
		assert_eq!(GetLastError(), ERROR_INVALID_PRINTER_COMMAND);
	}
}

#[cfg(windows)]
#[test]
fn test_win32_lifetimes() {
	use dylink::LazyFn;
	use std::sync::atomic::AtomicPtr;
	use windows_sys::Win32::Foundation::{ERROR_INVALID_PRINTER_COMMAND, WIN32_ERROR};
	use std::ffi::CStr;

	extern "stdcall" fn foo() -> WIN32_ERROR {
		// arbitrary data
		return ERROR_INVALID_PRINTER_COMMAND;
	}
	type PfnTy = extern "stdcall" fn () -> WIN32_ERROR;

	const FN_NAME: &'static CStr = unsafe {CStr::from_bytes_with_nul_unchecked(b"SetLastError\0")};

	let lazyfn = LazyFn::<PfnTy>::new(AtomicPtr::new(&mut (foo as PfnTy)));
	let old_ref = lazyfn.as_ref();
	let new_addr = lazyfn.load(FN_NAME, dylink::LinkType::System(&["Kernel32.dll\0"])).unwrap();
	
	assert_eq!(*old_ref, foo as PfnTy);	
	assert_ne!(new_addr, foo as PfnTy);	
	assert_ne!(lazyfn.as_ref(), old_ref);
	
}