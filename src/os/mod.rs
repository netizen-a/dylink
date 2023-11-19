// Copyright (c) 2023 Jonathan "Razordor" Alan Thomason
#[cfg_attr(docsrs, doc(cfg(unix)))]
#[cfg(any(unix, docsrs))]
pub mod unix;
#[cfg(windows)]
pub(crate) mod windows;
