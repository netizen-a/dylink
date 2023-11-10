#![cfg(unix)]

pub mod linux;
pub mod macos;

#[cfg(feature = "unstable")]
#[cfg(not(target_os = "aix"))]
#[test]
fn test_unix_sym_info() {
	use dylink::os::unix::SymExt;
	let this = Library::this();
	let symbol = this.symbol("atoi").unwrap();
	let info = symbol.info();
	println!("{:?}", info);
}
