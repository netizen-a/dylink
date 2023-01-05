use std::{error::Error, fmt};

#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
	FnNotFound,
	LibNotFound,
	ListNotFound
}

#[derive(Debug)]
pub struct DylinkError {
	subject: Option<&'static str>,
	pub(crate) kind: ErrorKind,
}

impl Error for DylinkError {}

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
				None => "function not found".to_owned()
			},
			ErrorKind::LibNotFound => match self.subject {
				Some(name) => format!("library `{name}` not found"),
				None => "library not found".to_owned(),
			},
			ErrorKind::ListNotFound => format!("libraries not found"),
		};
		write!(f, "Dylink Error: {err}")
	}
}
