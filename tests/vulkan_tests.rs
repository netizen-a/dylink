/// This test is not allowed to fail: This asserts that the vulkan is loaded properly in dylink.
#[test]
fn load_vulkan_dll() {
	use std::ffi::CStr;
	let vulkan_dll: &'static [u8] = if cfg!(windows) {
		b"vulkan-1.dll\0"
	} else if cfg!(target_os = "linux") {
		// the other way is the target "libvulkan.so"
		b"libvulkan.so.1\0"
	} else {
		// TODO: implement version for macOS.
		todo!()
	};
	let fn_name = CStr::from_bytes_with_nul(b"vkGetInstanceProcAddr\0").unwrap();
	let result = dylink::loader::loader(vulkan_dll, fn_name);
	if let Err(err) = result {
		panic!("{err}");
	}
}

/// This test is allowed to fail on potato PCs: vulkan 1.1 is required for this test to pass,
/// because `vkGetInstanceProcAddr` cannot load itself without an instance in vulkan 1.0
#[test]
fn load_vulkan_1_1() {
	use std::ffi::CStr;
	let vulkan_fn = CStr::from_bytes_with_nul(b"vkGetInstanceProcAddr\0").unwrap();

	let result = unsafe { dylink::loader::vkloader(None, vulkan_fn) };
	if let Err(err) = result {
		panic!("{err}");
	}
}
