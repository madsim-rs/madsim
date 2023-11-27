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

See also the blog posts for a detailed writeup:
- [Deterministic Simulation: A New Era of Distributed System Testing (Part 1 of 2)](https://www.risingwave.com/blog/deterministic-simulation-a-new-era-of-distributed-system-testing/)
- [Applying Deterministic Simulation: The RisingWave Story (Part 2 of 2)](https://www.risingwave.com/blog/applying-deterministic-simulation-the-risingwave-story-part-2-of-2/)

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
tonic = { version = "0.4", package = "madsim-tonic" }
etcd-client = { version = "0.4", package = "madsim-etcd-client" }
rdkafka = { version = "0.3", package = "madsim-rdkafka" }
aws-sdk-s3 = { version = "0.5", package = "madsim-aws-sdk-s3" }

[dev-dependencies]
tonic-build = { version = "0.4", package = "madsim-tonic-build" }
```

If your dependency graph includes the following crates, replace them by our patched version:

```toml
[patch.crates-io]
quanta = { git = "https://github.com/madsim-rs/quanta.git", rev = "948bdc3" }
getrandom = { git = "https://github.com/madsim-rs/getrandom.git", rev = "8daf97e" }
tokio-retry = { git = "https://github.com/madsim-rs/rust-tokio-retry.git", rev = "95e2fd3" }
tokio-postgres = { git = "https://github.com/madsim-rs/rust-postgres.git", rev = "4538cd6" }
tokio-stream = { git = "https://github.com/madsim-rs/tokio.git", rev = "ab251ad" }
```

When built normally, these crates are identical to the original ones.

To run your code on the simulator, enable the config `madsim`:

```sh
RUSTFLAGS="--cfg madsim" cargo test
```

Now you have gotten rid of tokio/tonic and you are in the simulation world!

We provide a set of APIs to control the simulator. You can use them to kill a process, disconnect the network, inject failures, etc.
Check out the [documentation](https://docs.rs/madsim) and search for the `madsim` feature to learn more usages.

## Projects

* [MadRaft](https://github.com/madsim-rs/madraft): The labs of Raft consensus algorithm derived from MIT 6.824 and PingCAP Talent Plan.
* [RisingWave](https://github.com/risingwavelabs/risingwave): A distributed SQL database for stream processing that uses MadSim for deterministic testing.

## License

Apache License 2.0
