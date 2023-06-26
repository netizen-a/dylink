# Dylink

![Crates.io](https://img.shields.io/crates/l/dylink) ![Crates.io](https://img.shields.io/crates/v/dylink) ![Crates.io](https://img.shields.io/crates/d/dylink) ![docs.rs](https://img.shields.io/docsrs/dylink) [![dylink-rs](https://github.com/Razordor/dylink/actions/workflows/rust.yml/badge.svg)](https://github.com/Razordor/dylink/actions/workflows/rust.yml) ![unsafe:yes](https://img.shields.io/badge/unsafe-yes-red)

Dylink provides a run-time dynamic linking framework for lazily evaluating shared libraries.
When functions are loaded they are evaluated through a thunk for first time calls, which loads the function from its respective library. Preceeding calls after initialization have no overhead or additional branching checks, since the thunk is replaced by the loaded function.

----

Related links:

* [Documentation](https://docs.rs/dylink)
* [Release notes](https://github.com/Razordor/dylink/releases)

## Optional Features

The crate comes with a variaty of useful features.

* `std` - enabled by default; Adds useful structures to use with the `Loader` trait.
  * If this feature is disabled, `no_std` is compatible.

* `unload` - enables support for unloading `SysLoader` defined libraries.
  * this enables unloading `SysLoader` loaded functions.
  * This feature is well defined, but considered super unsafe. Use at your own discretion.

## Supported platforms

Dylink has been implemented for all 3 major platforms.

| Windows | Linux | MacOS |
|:-------:|:-----:|:-----:|
| YES     | YES   | YES   |

## Usage

Add this to your `Cargo.toml`

```toml
[dependencies]
dylink = "0.7"
```

## Example

Below is a basic working example on how to use the macro on windows.

```rust
use dylink::*;
use std::ffi::CStr;

static KERNEL32: LazyLib<SysLoader, 1> = LazyLib::new([
   unsafe {CStr::from_bytes_with_nul_unchecked(b"Kernel32.dll\0")}
]);

#[dylink(library=KERNEL32)]
extern "stdcall" {
    fn GetLastError() -> u32;
    fn SetLastError(_: u32);
}

fn main() {
   unsafe {
      SetLastError(52);
      assert_eq!(52, GetLastError());
   }
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
