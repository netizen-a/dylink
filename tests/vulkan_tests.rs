#![allow(non_snake_case)]

use dylink::dylink;

// This test is not allowed to fail: This asserts that vulkan is loaded properly in dylink.
#[test]
fn test_vk_instance_layer_properties() {
	use std::ffi::c_char;
	type VkResult = i32;
	const VK_MAX_EXTENSION_NAME_SIZE: usize = 256;
	const VK_MAX_DESCRIPTION_SIZE: usize = 256;
	#[derive(Debug)]
	struct VkLayerProperties {
		_layerName: [c_char; VK_MAX_EXTENSION_NAME_SIZE],
		_specVersion: u32,
		_implementationVersion: u32,
		_description: [c_char; VK_MAX_DESCRIPTION_SIZE],
	}
	#[dylink(vulkan)]
	extern "system" {
		fn vkEnumerateInstanceLayerProperties(
			pPropertyCount: *mut u32,
			pProperties: *mut VkLayerProperties,
		) -> VkResult;
	}

	let mut property_count = 0;
	let mut properties;
	unsafe {
		let result = vkEnumerateInstanceLayerProperties(&mut property_count, std::ptr::null_mut());
		assert!(result >= 0);
		properties = Vec::with_capacity(property_count as usize);
		let result =
			vkEnumerateInstanceLayerProperties(&mut property_count, properties.as_mut_ptr());
		assert!(result >= 0);
		properties.set_len(property_count as usize);
	}
	for prop in properties {
		println!("{prop:?}");
	}
}
