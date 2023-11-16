#![cfg(unix)]

pub mod linux;
pub mod macos;

#[cfg(not(target_os = "aix"))]
#[test]
fn test_unix_sym_info() {
	use dylink::os::unix::SymExt;
	let this = dylink::Library::this();
	let symbol = this.symbol("atoi").unwrap();
	let info = symbol.info();
	assert!(info.is_ok());
}
