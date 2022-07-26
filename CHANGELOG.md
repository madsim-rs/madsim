# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-alpha.5] - 2022-07-26

## Added

- Migrate a new crate `madsim-tokio-postgres` for simulation.
- madsim-tokio: Add `task_local`, `task::LocalKey`, `signal::ctrl_c`.
- madsim-tonic: Support `Request::remote_addr` and returning error from server.

## Fixed

- madsim: Refactor TCP simulator and fix several bugs. (#18)

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
