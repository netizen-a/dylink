#[cfg(windows)]
#[test]
fn test_win32_kernel32() {
	use dylink::dylink;
	#[dylink(name = "Kernel32.dll")]
	extern "stdcall" {
		fn SetLastError(_: u32);
		fn GetLastError() -> u32;
	}
	unsafe {
		SetLastError(53);
		assert_eq!(GetLastError(), 53);
	}
}

// tbh I don't know why this test passes.
#[cfg(windows)]
#[test]
fn test_win32_lifetimes() {
	use dylink::LazyFn;
	use std::sync::atomic::AtomicPtr;
	use std::ffi::CStr;

	extern "stdcall" fn foo() -> u32 { 0 }
	type PfnTy = extern "stdcall" fn () -> u32;

	const FN_NAME: &'static CStr = unsafe {CStr::from_bytes_with_nul_unchecked(b"SetLastError\0")};

	let lazyfn = LazyFn::<PfnTy>::new(AtomicPtr::new(&mut (foo as PfnTy)));
	let old_ref = lazyfn.as_ref();
	let new_addr = lazyfn.load(FN_NAME, dylink::LinkType::System(&["Kernel32.dll"])).unwrap();
	
	assert_eq!(*old_ref, foo as PfnTy);	
	assert_ne!(new_addr, foo as PfnTy);	
	assert_ne!(lazyfn.as_ref(), old_ref);	
}