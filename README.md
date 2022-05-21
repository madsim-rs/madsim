# MadSim

[![Crate](https://img.shields.io/crates/v/madsim.svg)](https://crates.io/crates/madsim)
[![Docs](https://docs.rs/madsim/badge.svg)](https://docs.rs/madsim)
[![CI](https://github.com/madsys-dev/madsim/workflows/CI/badge.svg?branch=main)](https://github.com/madsys-dev/madsim/actions)

Magical Deterministic Simulator for distributed systems.

## Deterministic Simulation Testing

MadSim is a Rust async runtime similar to tokio, but with a key feature called **deterministic simulation**.

The main idea is borrowed from [FoundationDB](https://www.youtube.com/watch?v=4fFDFbi3toc) and [sled simulation guide](https://sled.rs/simulation.html).
Your code should be able to deterministically executed on top of a simulator.
The simulator will amplify randomness, create chaos and inject failures into your system.
A lot of hidden bugs may be revealed, which you can then deterministically reproduce them until they are fixed.
If your system can survive such chaos, you will have more confidence in deploying your system in the real world.

However, implementing deterministic simulation is difficult.
All I/O-related interfaces must be mocked during the simulation, and all uncertainties should be eliminated.
This project is created to make that easy.

A part of the implementation of this crate is inspired by [tokio-rs/simulation](https://github.com/tokio-rs/simulation).

## Usage

**NOTE: The current version 0.2 is in preview. API may be changed in the future.**

Add the following lines to your Cargo.toml:

```toml
[dependencies]
madsim = "0.2.0-alpha.2"
```

If your project depends on tokio or tonic, replace all `tokio`, `tonic` and `tonic-build` entries too:

```toml
[dependencies]
tokio = { version = "0.2.0-alpha.2", package = "madsim-tokio" }
tonic = { version = "0.2.0-alpha.2", package = "madsim-tonic" }

[dev-dependencies]
tonic-build = { version = "0.2.0-alpha.1", package = "madsim-tonic-build" }
```

Next, redirect the following APIs to madsim:

```rust
use std::collections    -> use madsim::collections
use rand                -> use madsim::rand
```

When built normally, these APIs are identical to the original ones.

To test your code on the simulator, enable the `sim` feature of madsim:

```sh
cargo test --features=madsim/sim
# or the `sim` feature of tokio if you use it
cargo test --features=tokio/sim
# or the `sim` feature of tonic if you use it
cargo test --features=tonic/sim
```

Now you have gotten rid of tokio/tonic and you are in the simulation world!

We provide a set of APIs to control the simulator. You can use them to kill a process, disconnect the network, inject failures, etc.
Check out the [documentation](https://docs.rs/madsim) and search for the `sim` feature to learn more usages.

### Ensure determinism

Developers should eliminate any randomness in the application code. That's not easy.

Here are some tips to avoid randomness:

* Use [`futures::select_biased`][select_biased] instead of [`futures::select`][select] macro.

[select_biased]: https://docs.rs/futures/0.3.21/futures/macro.select_biased.html
[select]: https://docs.rs/futures/0.3.21/futures/macro.select.html

To make sure your code is deterministic, run your test with the following environment variable:

```sh
MADSIM_TEST_CHECK_DETERMINISM=1
```

Your test will be run at least twice with the same seed. If any non-determinism detected, it will panic as soon as possible.

## Related Projects

* [MadRaft](https://github.com/madsys-dev/madraft): The labs of Raft consensus algorithm derived from MIT 6.824 and PingCAP Talent Plan.

## License

Apache License 2.0
