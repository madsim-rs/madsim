# madsim-tonic-build

[![Crate](https://img.shields.io/crates/v/madsim-tonic-build.svg)](https://crates.io/crates/madsim-tonic-build)
[![Docs](https://docs.rs/madsim-tonic-build/badge.svg)](https://docs.rs/madsim-tonic-build)

Compiles proto files via prost and generates service stubs and proto definitiones for use with madsim-tonic.

This crate will generate code for simulation along with the original code.
The macro `madsim_tonic::include_proto` will decide which version to use based on whether the `sim` feature is enabled.

This code is modified from [tonic-build v0.9.2][tonic-build]. It provides exactly the same API as the original crate.

[tonic-build]: https://github.com/hyperium/tonic/tree/v0.9.2/tonic-build
