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
