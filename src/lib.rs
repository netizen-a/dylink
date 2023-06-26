// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(doc, feature(doc_auto_cfg))]
#![cfg_attr(doc, feature(doc_cfg))]

mod lazylib;
/// custom linker module
mod loader;
#[cfg(feature = "std")]
mod os;

extern crate alloc;

pub use lazylib::*;
pub use loader::*;

/// Macro for generating dynamically linked functions procedurally.
///
/// Refer to crate level documentation for more information.
pub use dylink_macro::dylink;

/// Raw function address.
///
/// Must be cast into a function pointer to be useable.
pub type FnAddr = *const ();

#[doc = include_str!("../README.md")]
#[cfg(all(doctest, windows, feature = "std"))]
struct ReadmeDoctests;

#[cfg(not(target_has_atomic = "ptr"))]
compile_error!(
	"`AtomicPtr` is missing from this platform. `dylink` cannot function without this type."
);
