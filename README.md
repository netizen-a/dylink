# Dylink

## Overview

Dylink is a framework for lazily evaluating shared libraries such as `.dll` files for windows and `.so` files for unix.
When functions are loaded they are evaluated through a thunk for first time calls, which loads the function from it's
respective library. Proceeding calls after initialization have no overhead or additional branching checks, as the thunk is
replaced by the loaded function.

## Usage

Add this to your `Cargo.toml`

```toml
[dependencies]
dylink = "0.1"
```

<br>

#### License

<sub>
Licensed under either of

* Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
</sub>

#### Contribution

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
</sub>
