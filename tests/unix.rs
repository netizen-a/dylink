#![cfg(unix)]

#[cfg(not(target_os = "aix"))]
#[test]
fn test_unix_sym_info() {
	use dylink::os::unix::SymExt;
	use dylink::Symbol;
	let this = dylink::Library::this();
	let symbol = this.symbol("atoi").unwrap();
	let info = Symbol::info(symbol);
	assert!(info.is_ok());
}
