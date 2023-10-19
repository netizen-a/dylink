# Dylink

![Crates.io](https://img.shields.io/crates/l/dylink) ![Crates.io](https://img.shields.io/crates/v/dylink) ![Crates.io](https://img.shields.io/crates/d/dylink) ![docs.rs](https://img.shields.io/docsrs/dylink) [![dylink-rs](https://github.com/Razordor/dylink/actions/workflows/rust.yml/badge.svg)](https://github.com/Razordor/dylink/actions/workflows/rust.yml)

Dylink provides a run-time dynamic linking framework for loading dynamic libraries.

----

Related links:

* [Documentation](https://docs.rs/dylink)
* [Release notes](https://github.com/Razordor/dylink/releases)

## Features

* Thread-safe library loading.

## Supported platforms

Implemented for all major platforms.

| Windows | Linux | MacOS |
|:-------:|:-----:|:-----:|
| YES     | YES   | YES   |

## Usage

Add this to your `Cargo.toml`

```toml
[dependencies]
dylink = "0.8"
```

## Example

Below is a basic working example on how to use the macro on windows.

```rust
use dylink::*;

static KERNEL32: sync::LibLock = sync::LibLock::new(&["Kernel32.dll"]);

#[dylink(library=KERNEL32)]
extern "system-unwind" {
    fn GetLastError() -> u32;
    fn SetLastError(_: u32);
}

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
