use dylink::*;

#[test]
fn test_this_path() {
	let lib = Library::this();
	let path = lib.path();
	assert!(path.is_ok());
}

#[test]
fn test_try_clone() {
	let lib = Library::this();
	let other = lib.try_clone().unwrap();
	assert!(Image::ptr_eq(&lib, &other));
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
		print!("weak addr: {:p}, ", weak.as_ptr());
		if let Some(dylib) = weak.upgrade() {
			println!("upgraded = {}", dylib.path().unwrap().display());
			assert_eq!(weak.as_ptr(), dylib.as_ptr());
			assert_eq!(weak.path().ok(), dylib.path().ok());
		} else {
			println!("upgrade failed = {}", weak.path().unwrap().display());
		}
	}
}

// test to see if there are race conditions when getting a path.
#[test]
fn test_path_soundness() {
	use dylink::iter::Images;
	let images = Images::now().unwrap();
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
		let _ = lib.path().unwrap();
	}
	t.join().unwrap();
}


#[test]
fn test_magic() {
	use dylink::Image;
	let images = iter::Images::now().unwrap();
	for img in images {
		let magic = img.magic();
		if magic.is_null() {
			continue;
		}

		let magic = unsafe {&*magic};
		if cfg!(windows) {
			assert!(magic == [b'M', b'Z'] || magic == [b'Z', b'M'])
		} else if cfg!(target_os = "macos") {
			const MH_MAGIC: u32 = 0xfeedface;
			const MH_MAGIC_64: u32 = 0xfeedfacf;
			assert!(magic == MH_MAGIC.to_le_bytes() || magic == MH_MAGIC_64.to_le_bytes())
		} else if cfg!(unix) {
			const EI_MAG: [u8; 4] = [0x7f, b'E', b'L', b'F'];
			assert_eq!(magic, EI_MAG);
		}

	}
}