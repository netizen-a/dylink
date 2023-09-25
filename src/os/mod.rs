#[cfg(any(windows, docsrs))]
pub mod windows;
#[cfg(any(unix, docsrs))]
pub mod unix;