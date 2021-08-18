# MadSim

[![CI](https://github.com/madsys-dev/madsim/workflows/CI/badge.svg?branch=main)](https://github.com/madsys-dev/madsim/actions)

Magical Automatic Deterministic Simulator for distributed systems.

## Deterministic simulation

MadSim is a Rust async runtime similar to tokio, but with a key feature called **deterministic simulation**.

The main idea is borrowed from [sled simulation guide](https://sled.rs/simulation.html) and [FoundationDB](https://www.youtube.com/watch?v=4fFDFbi3toc). Part of the implementation is inspired by [tokio-rs/simulation](https://github.com/tokio-rs/simulation).

## Related Projects

* [MadRaft](https://github.com/madsys-dev/madraft): The labs of Raft consensus algorithm derived from MIT 6.824 and PingCAP Talent Plan.

## License

Apache License 2.0
