pub(crate) use std::ffi::{c_char, c_void, CStr, OsStr};

// FIXME: when extern types are stablized they must replace the `c_void` variation

// extern "C" {
// 	type VkInstance_T;
// 	type VkDevice_T;
// }

// #[repr(transparent)]
// #[derive(Clone, Copy, Eq, Hash, PartialEq)]
// pub struct VkInstance(pub(crate) *const VkInstance_T);

#[repr(transparent)]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct VkInstance(pub(crate) *const c_void);
unsafe impl Sync for VkInstance {}
unsafe impl Send for VkInstance {}

// #[repr(transparent)]
// #[derive(Clone, Copy, Eq, Hash, PartialEq)]
// pub struct VkDevice(pub(crate) *const VkDevice_T);

#[repr(transparent)]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct VkDevice(pub(crate) *const c_void);
unsafe impl Sync for VkDevice {}
unsafe impl Send for VkDevice {}
