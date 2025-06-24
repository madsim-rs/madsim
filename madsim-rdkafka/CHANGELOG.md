# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.4] - 2025-06-24

### Changed

- Enable background oauth token refresh (#243).

## [0.4.3] - 2025-01-09

### Changed

- Add `delete_groups` API to `AdminClient`. (no-op in simulation)

## [0.4.2] - 2024-05-13

### Changed

- Wrap `fetch_metadata`, `fetch_group_list` and `offsets_for_times` in `tokio::task::spawn_blocking`.

## [0.4.1] - 2024-05-08

### Fixed

- Add missing methods in `ClientContext`.
- Add missing `timeout` argument in `BaseConsumer::poll`.

## [0.4.0] - 2024-05-07

### Changed

- The associated constant `ClientContext::ENABLE_REFRESH_OAUTH_TOKEN` is changed to a function in order to make the trait object-safe.

## [0.3.4] - 2024-03-22

### Fixed

- Fix unintended drop of client in `fetch_watermarks`.

## [0.3.3] - 2024-02-28

### Changed

- Wrap `fetch_watermarks` in `tokio::task::spawn_blocking`.

## [0.3.2] - 2024-02-28

### Changed

- Update librdkafka to v2.3.0.

## [0.3.1] - 2024-01-05

### Fixed

- Add `rdkafka::message::{Header, HeaderIter}` and `BorrowedHeaders::detach`.
- Fix `rdkafka::message::Headers` trait.

## [0.3.0] - 2023-10-11

### Added

- Add statistics API.
- Add future producer API.

### Changed

- Update `rdkafka` to v0.34.0 and `librdkafka` to v2.2.0.

### Fixed

- Fix the error type of `DeliveryFuture`.

## [0.2.22] - 2023-04-19

### Added

- Add `producer::DeliveryResult` and fix `ProducerContext`.

## [0.2.14] - 2023-01-30

### Fixed

- Resolve DNS on connection.

## [0.2.13] - 2023-01-11

### Added

- Add `Timestamp::to_millis`.

### Changed

- update to rdkafka v0.29.0.
    - The return type of `Producer::flush` changed to `KafkaResult<()>`.

## [0.2.8] - 2022-09-26

### Added

- Add simulation crate of `rdkafka`.
