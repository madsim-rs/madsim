# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2024-07-12

### Changed

- Update `etcd-client` to v0.13.0.

## [0.4.0] - 2023-10-07

### Changed

- Update `etcd-client` to v0.12.1.

## [0.3.0] - 2023-07-20

### Changed

- Update `etcd-client` to v0.11.1.

## [0.2.23] - 2023-05-22

### Added

- Add `CampaignResponse::{take_header, take_leader}`.

### Fixed

- Fix the behavior when campaign is called multiple times.

## [0.2.20] - 2023-04-13

### Fixed

- Fix leak of RPC task in server.

## [0.2.18] - 2023-03-08

### Fixed

- Fix election implementation. Put a key for each candidate.

## [0.2.17] - 2023-02-17

### Fixed

- Support "etcdserver: request is too large".

## [0.2.15] - 2023-02-07

### Fixed

- Return error on "lease not found".

## [0.2.14] - 2023-01-30

### Fixed

- Resolve DNS on connection.

## [0.2.13] - 2023-01-11

### Added

- Add `KeyValue::{lease, create_revision, mod_revision}` API.
- Add maintenance `status` API.

### Fixed

- Fix response stream of `keep_alive`.
- Fix waking up other candidates on leadership resign or lease revoke.
- Fix unimplemented election `observe`.

## [0.2.11] - 2022-12-02

### Added

- Support load and dump in toml format.

### Fixed

- Fix lease grant.

## [0.2.8] - 2022-09-26

### Added

- Add lease and election API.

## [0.2.6] - 2022-09-05

### Added

- Add logging in etcd service.

### Fixed

- Complete `Error` type and fix the error kind of `request timed out`.

## [0.2.5] - 2022-09-02

### Added

- Add simulation crate of `etcd-client`.
