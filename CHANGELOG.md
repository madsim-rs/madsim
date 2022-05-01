# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
