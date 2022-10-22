use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum ErrCause {
    AlreadyLinked,
    FnNotFound,
}

#[derive(Debug)]
pub struct DylinkError {
    fn_name: &'static str,
    cause: ErrCause,
}
impl DylinkError {
    pub const fn new(fn_name: &'static str, cause: ErrCause) -> Self {
        Self {
            fn_name,
            cause,
        }
    }
}

impl Error for DylinkError {}

impl fmt::Display for DylinkError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ErrCause::*;
        let fn_name = self.fn_name;
        let err = match self.cause {
            AlreadyLinked => format!("function `{fn_name}` already linked"),
            FnNotFound => format!("function `{fn_name}` not found"),
        };
        write!(f, "Dylink Error: {err}")
    }
}