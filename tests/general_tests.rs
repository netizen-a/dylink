use dylink::dylink;

#[test]
fn test_dll_panic() {
    #[dylink(name = "../test_dll_panic.dll")]
    extern "Rust" {
        fn foo();
    }
    let result = std::panic::catch_unwind(|| unsafe{
        foo();
    });
    assert!(result.is_err());
}