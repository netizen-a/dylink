// SPDX-FileCopyrightText: 2022-2026 Jonathan A. Thomason <contact@jonathan-thomason.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(unix)]

#[cfg(not(target_os = "aix"))]
#[test]
fn test_unix_sym_info() {
	use dylink::Symbol;
	use dylink::os::unix::SymExt;
	let this = dylink::Library::this();
	let symbol = this.symbol("atoi").unwrap();
	let info = Symbol::info(symbol);
	assert!(info.is_ok());
}
