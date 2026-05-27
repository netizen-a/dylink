// SPDX-FileCopyrightText: 2022-2026 Jonathan A. Thomason <contact@jonathan-thomason.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

// not sure why this is deadcode on windows
#[cfg_attr(target_os = "windows", allow(dead_code))]
pub trait Sealed {}

#[cfg(windows)]
#[repr(C)]
pub struct Opaque {
	_data: [u8; 0],
	_marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

#[cfg(windows)]
impl Sealed for Opaque {}
