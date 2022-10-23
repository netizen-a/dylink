extern crate self as dylink;
use std::{ffi::c_void, os::raw::c_char};
use crate::FnPtr;

// Used in loader.rs, but should work as a good example case too.
// General case is used because it would cause recursion with vkloader.
#[dylink_macro::dylink(name = "vulkan-1")]
extern "system" {
	pub fn vkGetInstanceProcAddr(_: *const c_void, _: *const c_char) -> FnPtr;
}
