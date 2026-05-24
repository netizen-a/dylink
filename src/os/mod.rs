// SPDX-FileCopyrightText: 2022-2026 Jonathan A. Thomason <contact@jonathan-thomason.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

#[cfg_attr(docsrs, doc(cfg(unix)))]
#[cfg(any(unix, docsrs))]
pub mod unix;
#[cfg(windows)]
pub(crate) mod windows;
