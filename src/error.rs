use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum ErrorKind {
    AlreadyLinked,
    FnNotFound,
    LibNotFound,
}

#[derive(Debug)]
pub struct DylinkError {
    subject: String,
    kind: ErrorKind,
}
impl DylinkError {
    pub const fn new(subject: String, kind: ErrorKind) -> Self {
        Self {
            subject,
            kind,
        }
    }
}

impl Error for DylinkError {}

impl fmt::Display for DylinkError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ErrorKind::*;
        let err = match self.kind {
            AlreadyLinked => "function already linked",
            FnNotFound => "function not found",
            LibNotFound => "library not found",
        };
        write!(f, "Dylink Error: `{}` {err}", self.subject)
    }
}

pub type Result<T> = std::result::Result<T, DylinkError>;