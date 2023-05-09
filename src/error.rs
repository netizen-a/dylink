// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use std::{error, fmt};

/// The error enumeration dylink uses to define the error status.
///
/// This error structure may propagate from a dylink'd function generated from [dylink](crate::dylink).
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum DylinkError {
	/// The library was loaded, but the function was not.
	FnNotFound(String),
	/// The library was not loaded.
	LibNotLoaded(String),
	/// All the libraries were not loaded.
	ListNotLoaded(Vec<String>),
}

impl error::Error for DylinkError {}

impl fmt::Display for DylinkError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let err = match &self {
			Self::FnNotFound(fn_name) => format!("function `{fn_name}` not found"),
			Self::LibNotLoaded(lib_name) => format!("library `{lib_name}` could not be loaded"),
			Self::ListNotLoaded(msgs) => {
				let mut message = String::new();
				for m in msgs.iter() {
					message.push_str(&format!("{m}\n"));
				}
				// This makes the formatting slightly less recursive looking.
				return write!(f, "Dylink Error(s):\n{message}");
			}
		};
		write!(f, "Dylink Error: {err}")
	}
}
