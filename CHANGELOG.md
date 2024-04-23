# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Fix unaligned pointer access in getrandom.

## tokio [0.2.25] - 2024-04-08

### Removed

- Remove `stats` feature to allow tokio >=1.33.

## madsim [0.2.27] - 2024-04-07

### Fixed

- Fix the problem that `getrandom` returns different values in multiple runs with the same seed.

## rdkafka [0.3.4] - 2024-03-22

### Fixed

- Fix unintended drop of client in `fetch_watermarks`.

## madsim [0.2.26] - 2024-03-18

### Fixed

- `sleep` and `sleep_until` now sleep for at least 1ms to be consistent with tokio's behavior.

## rdkafka [0.3.3] - 2024-02-28

### Changed

- Wrap `fetch_watermarks` in `tokio::task::spawn_blocking`.

## rdkafka [0.3.2] - 2024-02-28

### Changed

- Update librdkafka to v2.3.0.

## tonic [0.4.2] tonic-build [0.4.3] - 2024-02-27

### Changed

- Allow `tonic` and `tonic-build` v0.11.

## madsim [0.2.25] - 2024-02-26

### Added

- Add `task::AbortHandle::is_finished`.

## madsim [0.2.24] - 2024-01-05

### Fixed

- Fix intercepting time on x86_64 macOS, Rust 1.75.0.

## rdkafka [0.3.1] - 2024-01-05

### Fixed

- Add `rdkafka::message::{Header, HeaderIter}` and `BorrowedHeaders::detach`.
- Fix `rdkafka::message::Headers` trait.

## aws-sdk-s3 [0.4.0] - 2023-11-24

### Changed

- Update `aws-sdk-s3` to v0.39.0.

## aws-sdk-s3 [0.3.0] - 2023-11-10

### Changed

- Update `aws-sdk-s3` to v0.35.0.

## madsim [0.2.23] - 2023-10-27

### Fixed

- madsim: Return an error instead of panicking when `lookup_host` receives an invalid address.

## tokio [0.2.24] - 2023-10-26

### Added

- tokio: Add `Runtime::enter` API.

## rdkafka [0.3.0] - 2023-10-11

### Added

- Add statistics API.
- Add future producer API.

### Changed

- Update `rdkafka` to v0.34.0 and `librdkafka` to v2.2.0.

### Fixed

- Fix the error type of `DeliveryFuture`.

## tonic-build [0.4.2] - 2023-10-08

### Added

- Add missing APIs for `Builder` in v0.10.0.

## etcd [0.4.0] - 2023-10-07

### Changed

- etcd: Update `etcd-client` to v0.12.1.

## tonic-build [0.4.1] - 2023-10-07

### Fixed

- Fix missing `futures_core` dependency.

## tonic [0.4.0] - 2023-09-13

### Changed

- tonic: Update `tonic` and `tonic-build` to v0.10.0.

## tonic [0.3.1] - 2023-07-24

### Added

- tonic: Add dummy methods `max_encoding_message_size` and `max_decoding_message_size` in `client::Grpc` and generated clients and servers.

### Changed

- tonic-build: Update methods `send_gzip` -> `send_compressed`, `accept_gzip` -> `accept_compressed` in generated clients and servers.

## tonic, etcd [0.3.0] - 2023-07-20

### Changed

- tonic: Update `tonic` and `tonic-build` to v0.9.2.
- etcd: Update `etcd-client` to v0.11.1.

## [0.2.25] - 2023-07-03

### Added

- tonic: Support setting timeout for each request.

### Fixed

- s3: Fix compile error.

## [0.2.24] - 2023-06-26

### Fixed

- s3: Fix missing parts ordering.

## [0.2.23] - 2023-05-22

### Added

- etcd: Add `CampaignResponse::{take_header, take_leader}`.
- tokio: Add `tokio::task::futures::TaskLocalFuture`.

### Changed

- s3: Update `aws-sdk-s3` to v0.28.

### Fixed

- etcd: Fix the behavior when campaign is called multiple times.


## [0.2.22] - 2023-04-19

### Added

- madsim: Add `restart_on_panic_matching` to support auto restarting on panic with certain messages.
- rdkafka: Add `producer::DeliveryResult` and fix `ProducerContext`.


## [0.2.21] - 2023-04-14

### Added

- madsim: Add `time::advance`.

### Fixed

- tonic: Fix panic on bi-directional streaming server closed.

## [0.2.20] - 2023-04-13

### Added

- madsim: Add detailed metrics for tasks.

### Changed

- madsim: Avoid spawning tasks for connection.

### Fixed

- madsim: Fix leak of task introduced in 0.2.19.
- etcd,tonic: Fix leak of RPC task in server.

## [0.2.19] - 2023-04-07

### Added

- madsim: Print context information on panic.
- madsim: Add `AbortHandle`.
- madsim: Add `RuntimeMetrics`.
- tokio: Support dropping `Runtime`.

### Changed

- madsim: Futures of a killed node are dropped after a while.

## [0.2.18] - 2023-03-08

### Added

- tonic: Support request timeout.

### Fixed

- madsim: Replace `SmallRng` with `Xoshiro256PlusPlus` for reproducibility across platforms.
- etcd: Fix election implementation. Put a key for each candidate.
- tonic: Return an error when the server stream is broken.

## [0.2.17] - 2023-02-17

### Fixed

- madsim: Prevent deadlock when killing a node.
- etcd: Support "etcdserver: request is too large".
- s3: Make fields public for `*Output` structs.

## [0.2.16] - 2023-02-14

### Fixed

- madsim: Fix `clock_gettime(CLOCK_BOOTTIME = 7)` on Linux.
- s3: Don't return error when deleted object is not found.

## [0.2.15] - 2023-02-07

### Added

- madsim: Add `buggify`.
- madsim: Add `JoinHandle::{id, is_finished}`.
- madsim: Add `signal::ctrl_c` and `Handle::send_ctrl_c`.
- madsim: Add `Handle::is_exit`.
- Add S3 simulator.

### Changed

- madsim: After the initial task completes, the other tasks of the node are dropped.

### Fixed

- etcd: Return error on "lease not found".

## [0.2.14] - 2023-01-30

### Added

- madsim: Add `NetSim::add_dns_record`.
- madsim: Add a global IPVS for load balancing.

### Fixed

- tonic/etcd/rdkafka: Resolve DNS on connection.

## [0.2.13] - 2023-01-11

### Added

- etcd: Add `KeyValue::{lease, create_revision, mod_revision}` API.
- etcd: Add maintenance `status` API.
- rdkafka: Add `Timestamp::to_millis`.

### Changed

- rdkafka: update to rdkafka v0.29.0.
    - The return type of `Producer::flush` changed to `KafkaResult<()>`.

### Fixed

- madsim: Fix join cancelled tasks.
- etcd: Fix response stream of `keep_alive`.
- etcd: Fix waking up other candidates on leadership resign or lease revoke.
- etcd: Fix unimplemented election `observe`.

## [0.2.12] - 2022-12-13

### Changed

- madsim: No longer initialize the global logger on `#[main]` or `#[test]`.

### Fixed

- madsim: Fix `Instant` interception on ARM64 macOS caused by change in Rust nightly.

## [0.2.11] - 2022-12-02

### Added

- tokio: Add `task::Builder::new_current_thread` but panic inside.
- tonic: Add `service` module and `Extensions`.
- tonic: Support interceptor.
- etcd: Support load and dump in toml format.

### Fixed

- tonic: Fix passing metadata in request and response. Add `content-type` and `date` field.
- tonic: Fix panic on unimplemented error.
- etcd: Fix lease grant.

## [0.2.10] - 2022-11-09

### Fixed

- madsim: Fix panic on TLS access error.

## [0.2.9] - 2022-10-28

### Fixed

- madsim: Fix internal structure change of nightly `std::time::{SystemTime, Interval}`.

## [0.2.8] - 2022-09-26

### Added

- Add simulation crate of `rdkafka`.
- etcd: Add lease and election API.
- madsim: Expose `JoinHandle::cancel_on_drop`.

## [0.2.7] - 2022-09-13

### Added

- tokio: Add fake `Runtime`.

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
