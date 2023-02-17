# MadSim

[![Crate](https://img.shields.io/crates/v/madsim.svg)](https://crates.io/crates/madsim)
[![Docs](https://docs.rs/madsim/badge.svg)](https://docs.rs/madsim)
[![CI](https://github.com/madsim-rs/madsim/workflows/CI/badge.svg?branch=main)](https://github.com/madsim-rs/madsim/actions)

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

Add the following lines to your Cargo.toml:

```toml
[dependencies]
madsim = "0.2"
```

If your project depends on the following crates, replace them by our simulators:

```toml
[dependencies]
tokio = { version = "0.2", package = "madsim-tokio" }
tonic = { version = "0.2", package = "madsim-tonic" }
etcd-client = { version = "0.2", package = "madsim-etcd-client" }
rdkafka = { version = "0.2", package = "madsim-rdkafka" }
aws-sdk-s3 = { version = "0.2", package = "madsim-aws-sdk-s3" }

[dev-dependencies]
tonic-build = { version = "0.2", package = "madsim-tonic-build" }
```

If your dependency graph includes the following crates, replace them by our patched version:

```toml
[patch.crates-io]
quanta = { git = "https://github.com/madsim-rs/quanta.git", rev = "a819877" }
getrandom = { git = "https://github.com/madsim-rs/getrandom.git", rev = "cc95ee3" }
tokio-retry = { git = "https://github.com/madsim-rs/rust-tokio-retry.git", rev = "95e2fd3" }
tokio-postgres = { git = "https://github.com/madsim-rs/rust-postgres.git", rev = "1b392f1" }
tokio-stream = { git = "https://github.com/madsim-rs/tokio.git", rev = "0c25710" }
```

When built normally, these crates are identical to the original ones.

To run your code on the simulator, enable the config `madsim`:

```sh
RUSTFLAGS="--cfg madsim" cargo test
```

Now you have gotten rid of tokio/tonic and you are in the simulation world!

We provide a set of APIs to control the simulator. You can use them to kill a process, disconnect the network, inject failures, etc.
Check out the [documentation](https://docs.rs/madsim) and search for the `madsim` feature to learn more usages.

### Ensure determinism

Developers should eliminate any randomness in the application code. That's not easy.

To make sure your code is deterministic, run your test with the following environment variable:

```sh
MADSIM_TEST_CHECK_DETERMINISM=1
```

Your test will be run at least twice with the same seed. If any non-determinism detected, it will panic as soon as possible.

## Related Projects

* [MadRaft](https://github.com/madsim-rs/madraft): The labs of Raft consensus algorithm derived from MIT 6.824 and PingCAP Talent Plan.

## License

Apache License 2.0
