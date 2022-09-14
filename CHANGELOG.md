# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.7] - 2022-09-13

### Changed

- madsim: Change the default seed to the nanosecond of current time.
- madsim: Wait for a while after panicking before restart.

### Fixed

- madsim: Avoid closing socket of a restarted node.

## [0.2.6] - 2022-09-05

### Added

- madsim: Add hook function for RPC.
- etcd: Add logging in etcd service.

### Removed

- madsim: Deprecate `Network::(dis)connect(2)` functions. Rename them to `(un)clog_*`.

### Fixed

- etcd: Complete `Error` type and fix the error kind of `request timed out`.

## [0.2.5] - 2022-09-02

### Added

- Add simulation crate of `etcd-client`.

### Changed

- madsim: Forbid creating system thread in simulation.
- madsim: Drop futures on killing node.
- madsim: Make `Endpoint` clonable.

## [0.2.4] - 2022-08-25

### Fixed

- madsim: Fix panic from minstant crate on Linux virtual machine.
- tonic: Fix panic when server sends a response but client has closed the stream.

## [0.2.3] - 2022-08-24

### Fixed

- madsim: Fix `lookup_host` on 'localhost'.

## [0.2.2] - 2022-08-24

### Added

- madsim: Add `task::Builder` API.
- madsim: Support auto restarting a node on panic.

### Changed

- madsim: Rename `TaskNodeHandle` to `Spawner`.

### Removed

- madsim: Deprecate `spawn_blocking`.

## [0.2.1] - 2022-08-19

### Added

- madsim: Add basic `net::UdpSocket`.

### Changed

- madsim: Refactor network simulator with a new connection primitive.
- madsim: Migrate logging facility to `tracing`. Replace `env_logger` with `tracing-subscriber`.
- madsim: Migrate std lock to spin lock.
- tonic: Reduce dependencies in simulation build.

### Fixed

- madsim: Fix the address of `TcpStream` accepted from `TcpListener`.
- madsim: Fix the socket address space. A TCP and a UDP sockets can have the same address in a node.
- tonic: Close the stream when the connection is broken.

## [0.2.0] - 2022-08-10

### Added

- madsim-tonic: Add missing methods of `Endpoint` and `Server`.

### Fixed

- madsim-tokio: `#[tokio::main]` no longer requires madsim crate.

### Removed

- madsim: Remove `collections` module since we can use std's directly.

## [0.2.0-alpha.7] - 2022-08-05

### Added

- madsim: Allow user to set the number of CPU cores and simulate `std::thread::available_parallelism`.
- madsim: Add `MADSIM_TEST_JOBS` environment variable to set the number of jobs to run simultaneously.
- madsim: Introduce runtime `Builder`.
- madsim: Expose seed via `Handle::seed`.

### Fixed

- madsim: Fix the local address after bind `0.0.0.0`.
- madsim-tonic: Client connecting to an invalid address should return error.

### Removed

- madsim: Remove `Runtime::{enable_determinism_check, take_rand_log}`. Replaced by `check_determinism`.


## [0.2.0-alpha.6] - 2022-08-01

### Added

- Make deterministic on `rand` and `std::{collections::{HashMap, HashSet}, time::{SystemTime, Instant}}`.

    Exception: `rand` is not deterministic on linux. use `madsim::rand` instead.

### Changed

- madsim-macros: Every simulation now runs on a new thread to ensure the determinism.
- madsim-tonic: Update tonic to v0.8. Additional system protoc is required.


## [0.2.0-alpha.5] - 2022-07-26

### Added

- Migrate a new crate `madsim-tokio-postgres` for simulation.
- madsim-tokio: Add `task_local`, `task::LocalKey`, `signal::ctrl_c`.
- madsim-tonic: Support `Request::remote_addr` and returning error from server.

### Fixed

- madsim: Refactor TCP simulator and fix several bugs. (#18)
- madsim: Avoid some panics on panicking.

## [0.2.0-alpha.4] - 2022-07-18

### Added

- madsim: Add TCP simulation.
- madsim-tokio: Add `task::consume_budget` and `task::Id`.

### Fixed

- madsim: Avoid duplicate connection and close unused connection on TCP.
- madsim-tokio: Fix `full` feature.

## [0.2.0-alpha.3] - 2022-05-25

### Added

- madsim: Add serde API to `HashMap`.
- madsim-norandom: A preview library for intercepting libc `getrandom`.

### Changed

- **Breaking:** Change the way to enable simulation: `#[cfg(feature = "sim")]` -> `#[cfg(madsim)]`.

### Fixed

- Lock version on madsim dependencies to prevent API broken.

## [0.2.0-alpha.2] - 2022-05-22

### Added

- Add a new crate `madsim-tokio` for tokio simulation.
- madsim/sim: Add `time::interval` and `task::yield_now`.
- madsim/sim: Complete methods for `Sleep`, `Elapsed`, `JoinError` and `JoinHandle`.
- madsim-tonic: Add `Server::layer` but don't implement it.

### Changed

- **Breaking:** madsim: Switch `JoinHandle` to tokio style which won't cancel task on drop.
- madsim-macros: Improve error message on panic in simulation.

### Fixed

- madsim: Fix TCP performance issue by setting NODELAY.

## [0.2.0-alpha.1] - 2022-05-12

TODO

### Added

- A real world backend.

## [0.1.3] - 2021-11-30

### Fixed

- Fix deadlock on M1 macOS: add epsilon on time increasing

## [0.1.2] - 2021-11-04

### Added

- API to get the local socket address.

## [0.1.1] - 2021-08-24

### Added

- Deterministic check on test.

### Changed

- Remove default time limit (300s) on test.
- Improve error message on context missing.

### Fixed

- Fix double panic in the executor.
- Fix deterministic in `time::timeout`.

## [0.1.0] - 2021-08-22

### Added

- Deterministic async runtime.
- Basic simulated network and file system.
