# Dylink

![Crates.io](https://img.shields.io/crates/l/dylink) ![Crates.io](https://img.shields.io/crates/v/dylink) ![Crates.io](https://img.shields.io/crates/d/dylink) ![docs.rs](https://img.shields.io/docsrs/dylink) [![dylink-rs](https://github.com/Razordor/dylink/actions/workflows/rust.yml/badge.svg)](https://github.com/Razordor/dylink/actions/workflows/rust.yml)

Dylink provides a run-time dynamic linking framework for loading dynamic libraries.

This crate may be useful if the dynamic library you are loading is not always guarenteed
to exist, which may enable you to provide fallbacks in your code.

----

Related links:

* [Documentation](https://docs.rs/dylink)
* [Release notes](https://github.com/Razordor/dylink/releases)

## Features

* Thread-safe library loading.
* Macro attribute

## Supported platforms

Implemented for all major platforms.

| Windows | Linux | MacOS |
|:-------:|:-----:|:-----:|
| YES     | YES   | YES   |

## Usage

Add this to your `Cargo.toml`

```toml
[dependencies]
dylink = "0.10"
```

## Examples

Below is an example of opening a library manually through `Library` on Windows.

```rust
use dylink::*;
use std::mem;

// Open the Kernel32.dll library.
let lib = Library::open("Kernel32.dll").expect("Failed to open library");

// Get the symbol for the GetLastError function.
let sym = lib.symbol("GetLastError").unwrap();

// Cast the symbol to the appropriate function signature.
let get_last_error: unsafe extern "system" fn() -> u32 = unsafe {mem::transmute(sym)};

let result = unsafe {get_last_error()};

// Call the function and assert its return value.
assert_eq!(result, 0);
```

Below is an example on how to use the `dylink` attribute on Windows. This example demonstrates the
lazy loading capability of the `dylink` crate by interacting with functions from the Kernel32.dll library.

```rust
use dylink::*;

// Define a static LibLock for the Kernel32.dll library.
static KERNEL32: sync::LibLock = sync::LibLock::new(&["Kernel32.dll"]);

// Use the `dylink` attribute to declare functions from the Kernel32.dll.
#[dylink(library=KERNEL32)]
extern "system-unwind" {
    fn GetLastError() -> u32;
    fn SetLastError(_: u32);
}

// Use the declared functions, which will be loaded lazily when called.
unsafe {
   SetLastError(52);
   assert_eq!(52, GetLastError());
}
```

### License

Licensed under either of

* Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
