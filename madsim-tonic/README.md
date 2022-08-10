# madsim-tonic

[![Crate](https://img.shields.io/crates/v/madsim-tonic.svg)](https://crates.io/crates/madsim-tonic)
[![Docs](https://docs.rs/madsim-tonic/badge.svg)](https://docs.rs/madsim-tonic)

The `tonic` simulator on madsim.

> If it looks like tonic, acts like tonic, and is used like tonic, then it probably is tonic.

## Usage

Replace all `tonic` and `tonic-build` entries in your Cargo.toml:

```toml
[dependencies]
tonic = { version = "0.2", package = "madsim-tonic" }

[dev-dependencies]
tonic-build = { version = "0.2", package = "madsim-tonic-build" }
```
