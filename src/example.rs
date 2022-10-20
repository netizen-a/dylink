extern crate self as dylink;
use std::{ffi::c_void, os::raw::c_char};

use windows_sys::Win32::Foundation::PROC;

// Used in loader.rs, but should work as a good example case too.
#[dylink_macro::dylink(name = "vulkan-1")]
extern "system" {
	pub fn vkGetInstanceProcAddr(_: *const c_void, _: *const c_char) -> PROC;
}
