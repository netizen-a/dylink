#[test]
fn load_vulkan_lib() {
    use std::ffi::CStr;
    let vulkan_dll = CStr::from_bytes_with_nul(b"vulkan-1\0").unwrap();
    let fn_name = CStr::from_bytes_with_nul(b"vkGetInstanceProcAddr\0").unwrap();
    let result = dylink::loader::loader(vulkan_dll, fn_name);
    assert!(result.is_ok());    
}