//#![allow(unused_doc_comments)]

extern crate self as dylink;
use std::{ffi, os::raw::c_char};

use crate::FnPtr;

#[dylink_macro::dylink(name = "vulkan-1")]
extern "system" {
	pub unsafe fn vkGetInstanceProcAddr(
		instance: *const ffi::c_void,
		pName: *const c_char,
	) -> Option<FnPtr>;
}
