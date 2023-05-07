// This test is not allowed to fail: This asserts that vulkan is loaded properly in dylink.
// If vulkan drivers are not installed properly, then this test will fail regardless.
#[test]
fn test_vk_instance_layer_properties() {
	#![allow(non_snake_case)]
	use dylink::dylink;
	use std::ffi::c_char;

	type VkResult = i32;
	const VK_MAX_EXTENSION_NAME_SIZE: usize = 256;
	const VK_MAX_DESCRIPTION_SIZE: usize = 256;
	#[derive(Debug, Clone, Copy)]
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
		properties = vec![
			VkLayerProperties {
				_layerName: [0; VK_MAX_EXTENSION_NAME_SIZE],
				_specVersion: 0,
				_implementationVersion: 0,
				_description: [0; VK_MAX_DESCRIPTION_SIZE]
			};
			property_count as usize
		];
		let result =
			vkEnumerateInstanceLayerProperties(&mut property_count, properties.as_mut_ptr());
		assert!(result >= 0);
	}
}
