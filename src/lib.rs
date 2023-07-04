// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
#![cfg_attr(doc, feature(doc_auto_cfg))]
#![cfg_attr(doc, feature(doc_cfg))]

//! Dylink provides a run-time dynamic linking framework for lazily evaluating shared libraries.
//! When functions are loaded they are evaluated through a thunk for first time calls, which loads the function
//! from its respective library. Preceeding calls after initialization have no overhead or additional branching
//! checks, since the thunk is replaced by the loaded function.
//!
//! # Basic Example
//!
//! ```rust
//! use dylink::*;
//! use std::ffi::CStr;
//!
//! static KERNEL32: Library<SystemLoader> = Library::new(&["Kernel32.dll\0"]);
//!
//! #[dylink(library=KERNEL32)]
//! extern "system" {
//!     fn GetLastError() -> u32;
//!     fn SetLastError(_: u32);
//! }
//! ```

mod library;
mod loader;
mod os;

pub use library::*;
pub use loader::*;

/// Macro for generating shared symbol thunks procedurally.
///
/// Refer to crate level documentation for more information.
pub use dylink_macro::dylink;

/// Raw function address.
///
/// Must be cast into a function pointer to be useable.
pub type FnAddr = *const ();

#[doc = include_str!("../README.md")]
#[cfg(all(doctest, windows))]
struct ReadmeDoctests;

#[cfg(not(target_has_atomic = "ptr"))]
compile_error!(
	"`AtomicPtr` is missing from this platform. `dylink` cannot function without this type."
);
