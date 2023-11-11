use dylink::*;

#[test]
fn test_this_path() {
	let lib = Library::this();
	let path = lib.path().unwrap();
	println!("path = {}", path.display());
}

#[test]
fn test_this_metadata() -> std::io::Result<()> {
	let lib = Library::this();
	let metadata = lib.metadata()?;
	println!("metadata = {:?}", metadata);
	Ok(())
}

#[test]
fn test_try_clone() -> std::io::Result<()> {
	let lib = Library::this();
	let other = lib.try_clone()?;
	assert!(Image::ptr_eq(&lib, &other));
	let t = std::thread::spawn(move || {
		println!("other: {:?}", other);
	});
	t.join().unwrap();
	println!("lib: {:?}", lib);
	Ok(())
}

#[test]
fn test_iter_images() -> std::io::Result<()> {
	let images = iter::Images::now()?;
	for weak in images {
		print!("weak addr: {:p}, ", weak.as_ptr());
		if let Some(dylib) = weak.upgrade() {
			println!("upgraded = {}", dylib.path()?.display());
			assert_eq!(weak.as_ptr(), dylib.as_ptr());
			assert_eq!(weak.path().ok(), dylib.path().ok());
		} else {
			println!("upgrade failed = {}", weak.path()?.display());
		}
	}
	Ok(())
}

// test to see if there are race conditions when getting a path.
#[test]
fn test_path_soundness() -> std::io::Result<()> {
	use dylink::iter::Images;
	let images = Images::now()?;
	let mut vlib = vec![];
	for img in images {
		if let Some(val) = img.upgrade() {
			vlib.push(val)
		}
	}
	let t = std::thread::spawn(|| {
		let images = Images::now().unwrap();
		let mut other_vlib = vec![];
		for img in images {
			if let Some(val) = img.upgrade() {
				other_vlib.push(val)
			}
		}
		for lib in other_vlib.drain(0..) {
			let _ = lib.path().unwrap();
		}
	});
	for lib in vlib.drain(0..) {
		let _ = lib.path()?;
	}
	t.join().unwrap();
	Ok(())
}
