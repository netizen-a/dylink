use std::{error::Error, fmt};

#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
	FnNotFound,
	LibNotFound,
}

#[derive(Debug)]
pub struct DylinkError {
	subject:         String,
	pub(crate) kind: ErrorKind,
}

impl Error for DylinkError {}

impl DylinkError {
	#[inline]
	pub fn new(subject: String, kind: ErrorKind) -> Self { Self { subject, kind } }
}

impl fmt::Display for DylinkError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let subject = &self.subject;
		let err = match self.kind {
			ErrorKind::FnNotFound => format!("function `{subject}` not found"),
			ErrorKind::LibNotFound => format!("library `{subject}` not found"),
		};
		write!(f, "Dylink Error: {err}")
	}
}
