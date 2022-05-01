# MadSim

[![Crate](https://img.shields.io/crates/v/madsim.svg)](https://crates.io/crates/madsim)
[![Docs](https://docs.rs/madsim/badge.svg)](https://docs.rs/madsim)
[![CI](https://github.com/madsys-dev/madsim/workflows/CI/badge.svg?branch=main)](https://github.com/madsys-dev/madsim/actions)

Magical Automatic Deterministic Simulator for distributed systems.

## Deterministic simulation

MadSim is a Rust async runtime similar to tokio, but with a key feature called **deterministic simulation**.

The main idea is borrowed from [sled simulation guide](https://sled.rs/simulation.html) and [FoundationDB](https://www.youtube.com/watch?v=4fFDFbi3toc). Part of the implementation is inspired by [tokio-rs/simulation](https://github.com/tokio-rs/simulation).

### Ensure determinism

Developers should eliminate any randomness in the application code. That's not easy.

Here are some tips to avoid randomness:

* Use [`futures::select_biased`][select_biased] instead of [`futures::select`][select] macro.
* Do not iterate through a `HashMap`.

[select_biased]: https://docs.rs/futures/0.3.16/futures/macro.select_biased.html
[select]: https://docs.rs/futures/0.3.16/futures/macro.select.html

To make sure your code is deterministic, run your test with the following environment variable:

```sh
MADSIM_TEST_CHECK_DETERMINISM=1
```

Your test will be run at least twice with the same seed. If any non-determinism detected, it will panic as soon as possible.

## Related Projects

* [MadRaft](https://github.com/madsys-dev/madraft): The labs of Raft consensus algorithm derived from MIT 6.824 and PingCAP Talent Plan.

## License

Apache License 2.0
