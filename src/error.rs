// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use std::{error, fmt};

/// The error enumeration dylink uses to define the error status.
///
/// This error structure may propagate from a dylink'd function generated from [dylink](crate::dylink).
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
			Self::LibNotLoaded(err_msg) => format!("could not load library:{err_msg}"),
			// todo: print all error messages
			Self::ListNotLoaded(_) => "libraries not loaded".to_owned(),
		};
		write!(f, "Dylink Error: {err}")
	}
}
