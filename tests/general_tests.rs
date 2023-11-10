use dylink::*;

#[test]
fn test_this_path() {
	let lib = Library::this();
	let path = lib.path().unwrap();
	println!("path = {}", path.display());
}

#[test]
fn test_this_metadata() {
	let lib = Library::this();
	let metadata = lib.metadata();
	println!("metadata = {:?}", metadata);
}

#[test]
fn test_try_clone() {
	let lib = Library::this();
	let other = lib.try_clone().expect("failed to clone handle");
	assert_eq!(lib, other);
	let t = std::thread::spawn(move || {
		println!("other: {:?}", other);
	});
	t.join().unwrap();
	println!("lib: {:?}", lib);
}

#[test]
fn test_iter_images() {
	let images = iter::Images::now().unwrap();
	for weak in images {
		print!("weak addr: {:p}, ", weak.addr());
		if let Some(dylib) = weak.upgrade() {
			println!("upgraded = {}", dylib.path().unwrap().display());
			assert_eq!(weak.addr(), dylib.addr());
			assert_eq!(weak.path().ok(), dylib.path().ok());
		} else {
			println!("upgrade failed");
		}
	}
}
