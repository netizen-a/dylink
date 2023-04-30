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

	extern "stdcall" fn foo() -> u32 { 0 }
	type PfnTy = extern "stdcall" fn () -> u32;

	let lazyfn = LazyFn::<PfnTy>::new(AtomicPtr::new(&mut (foo as PfnTy)));
	let old_ref = lazyfn.as_ref();
	let new_addr = lazyfn.load("SetLastError", dylink::LinkType::System(&["Kernel32.dll"])).unwrap();
	
	assert_eq!(*old_ref, foo as PfnTy);	
	assert_ne!(new_addr, foo as PfnTy);	
	assert_ne!(lazyfn.as_ref(), old_ref);	
}