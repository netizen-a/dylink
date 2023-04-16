// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
use std::ffi;
// FIXME: when extern types are stablized they must replace the `c_void` variation

// extern "C" {
// 	type VkInstance_T;
// 	type VkDevice_T;
// }

// #[repr(transparent)]
// #[derive(Clone, Copy, Eq, Hash, PartialEq)]
// pub struct VkInstance(pub(crate) *const VkInstance_T);

#[doc(hidden)]
#[repr(transparent)]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct VkInstance(pub(crate) *const ffi::c_void);
unsafe impl Sync for VkInstance {}
unsafe impl Send for VkInstance {}

// #[repr(transparent)]
// #[derive(Clone, Copy, Eq, Hash, PartialEq)]
// pub struct VkDevice(pub(crate) *const VkDevice_T);

#[doc(hidden)]
#[repr(transparent)]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct VkDevice(pub(crate) *const ffi::c_void);
unsafe impl Sync for VkDevice {}
unsafe impl Send for VkDevice {}
