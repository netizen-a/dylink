#[test]
fn load_vulkan_dll() {
    use std::ffi::CStr;
    let vulkan_dll = if cfg!(windows) {
        CStr::from_bytes_with_nul(b"vulkan-1.dll\0").unwrap()
    } else if cfg!(unix) {
        CStr::from_bytes_with_nul(b"libvulkan.so.1\0").unwrap()
    } else {
        todo!()
    };
    let fn_name = CStr::from_bytes_with_nul(b"vkGetInstanceProcAddr\0").unwrap();
    let result = dylink::loader::loader(vulkan_dll, fn_name);
    if let Err(err) = result {
        panic!("{err}");
    }
}

#[test]
fn load_vulkan_fn() {
    use std::ffi::CStr;
    let vulkan_fn =  CStr::from_bytes_with_nul(b"vkGetInstanceProcAddr\0").unwrap();
    let result = unsafe {dylink::loader::vkloader(None, vulkan_fn)};
    if let Err(err) = result {
        panic!("{err}");
    }
}