#[cfg(not(miri))]
#[cfg(windows)]
#[test]
#[ignore = "this is just instrumentation"]
fn test_win32_alloc_instrumentation() {
	use dylink::*;
	use std::alloc::{self, GlobalAlloc, Layout};
	use std::sync::atomic::AtomicUsize;
	use std::sync::atomic::Ordering;

	struct MyAllocator(AtomicUsize);

	unsafe impl GlobalAlloc for MyAllocator {
		unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
			self.0.fetch_add(1, Ordering::SeqCst);
			alloc::System.alloc(layout)
		}

		unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
			alloc::System.dealloc(ptr, layout)
		}
	}

	#[global_allocator]
	static GLOBAL: MyAllocator = MyAllocator(AtomicUsize::new(0));
	static LIB: Library<SystemLoader> =
		Library::new(&["Kernel32.dll"]);

	// macro output: function
	#[dylink(library = LIB)]
	extern "C" {
		fn GetLastError() -> u32;
	}

	// factor in any allocs that aren't mine, take the difference and print it.
	unsafe {
		let test_allocs = GLOBAL.0.load(Ordering::Acquire);
		let _ = GetLastError();
		let dylink_allocs = GLOBAL.0.load(Ordering::Acquire) - test_allocs;
		println!("dylink allocations={dylink_allocs}");
	}
}
