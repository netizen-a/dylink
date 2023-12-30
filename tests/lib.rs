mod linux;
mod macos;
mod unix;
mod windows;

use dylink::*;

#[test]
fn test_try_clone() {
	let lib = Library::this();
	let other = lib.try_clone().unwrap();
	let lib_data = lib.to_image().unwrap().to_bytes().unwrap();
	let other_data = other.to_image().unwrap().to_bytes().unwrap();
	assert_eq!(lib_data, other_data);
	let t = std::thread::spawn(move || {
		println!("other: {:?}", other);
	});
	t.join().unwrap();
	println!("lib: {:?}", lib);
}

#[test]
fn test_iter_images() {
	let images = img::Images::now().unwrap();
	for weak in images {
		print!("weak addr: {:p}, ", weak.to_ptr());
		if let Some(dylib) = weak.upgrade() {
			let hdr = dylib.to_image().unwrap();
			if let Ok(path) = hdr.path() {
				println!("upgraded = {}", path.display());
				assert_eq!(path, dylib.to_image().unwrap().path().unwrap());
			}
			let weak_img = unsafe { weak.to_ptr().as_ref() }.unwrap();
			let weak_data = weak_img.to_bytes().unwrap();
			let hdr_data = hdr.to_bytes().unwrap();
			assert_eq!(weak_data, hdr_data);
		} else if let Some(path) = weak.path() {
			println!("upgrade failed = {}", path.display());
		}
	}
}

// test to see if there are race conditions when getting a path.
#[test]
fn test_path_soundness() {
	use dylink::img::Images;
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
			let _ = lib.try_clone().unwrap();
		}
	});
	for lib in vlib.drain(0..) {
		let _ = lib.try_clone().unwrap();
	}
	t.join().unwrap();
}

#[test]
fn test_hdr_path() {
	let images = img::Images::now().unwrap();
	for img in images {
		let maybe_hdr = unsafe { img.to_ptr().as_ref() };
		let Some(hdr) = maybe_hdr else {
			continue;
		};
		if let Some(path) = img.path() {
			assert_eq!(path, hdr.path().unwrap());
		}
	}
}

#[test]
fn test_downgrade_upgrade() {
	let strong = Library::this();
	let weak = Library::downgrade(&strong).unwrap();
	let strong_clone = weak.upgrade();

	assert!(strong_clone.is_some());
}
