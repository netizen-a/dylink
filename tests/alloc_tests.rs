

#[cfg(windows)]
#[test]
#[ignore = "this is just instrumentation"]
fn test_win32_alloc_instrumentation() {
    use dylink::*;
    use std::sync::atomic::Ordering;
    use std::sync::atomic::AtomicUsize;
    use std::alloc::{GlobalAlloc, System, Layout};

    struct MyAllocator(AtomicUsize);

    unsafe impl GlobalAlloc for MyAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            self.0.fetch_add(1, Ordering::SeqCst);
            System.alloc(layout)
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            System.dealloc(ptr, layout)
        }
    }

    #[global_allocator]
    static GLOBAL: MyAllocator = MyAllocator(AtomicUsize::new(0));

	// macro output: function
	#[dylink(name = "Kernel32.dll")]
	extern "C" {
		fn GetLastError() -> u32;
	}

	unsafe {		
        // value doesn't really matter, but print it anyway.
        let e = GetLastError();
        let n = GLOBAL.0.load(Ordering::Acquire);
        println!("GetLastError={e}\nallocations={n}");
	}
}