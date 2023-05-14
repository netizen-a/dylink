#[cfg(windows)]
use dylink::*;

// This isn't meant to be an all encompassing test, but just mere proof of concept.
#[cfg(windows)]
#[test]
fn test_win32_scope() {
	// macro output: static variable
	#[dylink(name = "Kernel32.dll", strip = true)]
	extern "stdcall" {
		fn GetLastError() -> u32;
	}

	std::thread::scope(|s| {
		s.spawn(move || match GetLastError.try_link() {
			Ok(f) => unsafe { f() },
			Err(e) => panic!("{}", e),
		});
		s.spawn(move || match GetLastError.try_link() {
			Ok(f) => unsafe { f() },
			Err(e) => panic!("{}", e),
		});
	});
}
