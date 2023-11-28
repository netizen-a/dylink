mod linux;
mod macos;
mod unix;
mod windows;

use dylink::*;

// This test generally fails on macos, but that's ok
#[cfg(not(target_os = "macos"))]
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
	assert_eq!(lib.to_header() as *const _, other.to_header() as *const _);
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
			println!("upgraded = {}", dylib.path().unwrap().display());
			assert_eq!(weak.to_ptr(), dylib.to_header());
			assert_eq!(weak.path(), dylib.path().ok().as_ref());
		} else {
			println!("upgrade failed = {}", weak.path().unwrap().display());
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
			let _ = lib.path().unwrap();
		}
	});
	for lib in vlib.drain(0..) {
		let _ = lib.path().unwrap();
	}
	t.join().unwrap();
}

#[test]
fn test_hdr_magic() {
	let images = img::Images::now().unwrap();
	for img in images {
		let maybe_hdr = unsafe { img.to_ptr().as_ref() };
		let Some(hdr) = maybe_hdr else {
			continue;
		};
		let magic = hdr.magic();
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

#[test]
fn test_hdr_bytes() {
	let images = img::Images::now().unwrap();
	for img in images {
		let maybe_hdr = unsafe { img.to_ptr().as_ref() };
		let Some(hdr) = maybe_hdr else {
			continue;
		};
		let bytes = hdr.to_bytes().unwrap();
		assert!(bytes.len() > 0);
		let _ = bytes[bytes.len() - 1];
	}
}


#[test]
fn test_hdr_path() {
	let images = img::Images::now().unwrap();
	for img in images {
		let maybe_hdr = unsafe { img.to_ptr().as_ref() };
		let Some(hdr) = maybe_hdr else {
			continue;
		};

		assert_eq!(img.path().unwrap(), &hdr.path().unwrap());
	}
	let this = Library::this();
	let this_ptr = this.to_header();

	assert_eq!(this.path().unwrap(), (&*this_ptr).path().unwrap())
}