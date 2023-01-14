use dylink::dylink;

#[test]
fn unwind_test() {
    use std::panic;
    
    #[dylink(name = "foobar")]
    extern "C" {
        fn bar();
    }
    
    let result = panic::catch_unwind(panic::AssertUnwindSafe(||{
        unsafe {
            bar();
        }
    }));
    assert!(result.is_err());
}
