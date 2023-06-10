// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

pub mod lazylib;
/// custom linker module
pub mod loader;

pub use lazylib::*;

mod os;

/// Macro for generating dynamically linked functions procedurally.
///
/// Refer to crate level documentation for more information.
pub use dylink_macro::dylink;

#[doc = include_str!("../README.md")]
#[cfg(all(doctest, windows))]
pub struct ReadmeDoctests;

/// Raw function address.
///
/// Must be cast into a function pointer to be useable.
pub type FnAddr = *const ();
