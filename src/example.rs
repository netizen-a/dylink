//#![allow(unused_doc_comments)]

extern crate self as dylink;
use core::ffi::c_char;

use crate::FnPtr;

#[dylink_macro::dylink(name = "vulkan-1")]
extern "system" {
	pub fn vkGetInstanceProcAddr(instance: *const std::ffi::c_void, pName: *const c_char) -> Option<FnPtr>;
}
