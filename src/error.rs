// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason

use std::{error, fmt};

/// An enumeration of the context of the error.
///
/// Used with [DylinkError].
#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
	/// Declares the library was found, but the function was not.
	FnNotFound,
	/// Declares the library was not found.
	LibNotFound,
	/// Declares all the libraries were not found.
	ListNotFound,
}

// TODO: document to use unwind friendly ABI for dealing with panics

/// The error structure dylink uses to define the error status.
///
/// This error structure may propagate from a dylink'd function generated from [dylink](crate::dylink).
/// You can check if the function panicked through [catch_unwind](std::panic::catch_unwind), however,
/// many [ABIs](https://doc.rust-lang.org/reference/items/external-blocks.html#abi) are not [UnwindSafe](std::panic::UnwindSafe).
/// It's ideal not to rely on unwinding unless you know for sure that the ABI you are using can unwind safely like `extern "Rust"`.
#[derive(Debug, Clone)]
pub struct DylinkError {
	subject: Option<&'static str>,
	pub(crate) kind: ErrorKind,
}

impl error::Error for DylinkError {}

impl DylinkError {
	#[inline]
	pub const fn new(subject: Option<&'static str>, kind: ErrorKind) -> Self {
		Self { subject, kind }
	}

	#[inline]
	pub const fn kind(&self) -> ErrorKind {
		self.kind
	}
}

impl fmt::Display for DylinkError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let err = match self.kind {
			ErrorKind::FnNotFound => match self.subject {
				Some(name) => format!("function `{name}` not found"),
				None => "function not found".to_owned(),
			},
			ErrorKind::LibNotFound => match self.subject {
				Some(name) => format!("library `{name}` not found"),
				None => "library not found".to_owned(),
			},
			ErrorKind::ListNotFound => "libraries not found".to_string(),
		};
		write!(f, "Dylink Error: {err}")
	}
}
