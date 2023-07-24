# madsim-rdkafka

[![Crate](https://img.shields.io/crates/v/madsim-rdkafka.svg)](https://crates.io/crates/madsim-rdkafka)
[![Docs](https://docs.rs/madsim-rdkafka/badge.svg)](https://docs.rs/madsim-rdkafka)

The `rdkafka` simulator on madsim. Mirrors [rdkafka v0.34.0](https://docs.rs/rdkafka/0.34.0/rdkafka/index.html) and librdkafka 2.2.0.

## Usage

Replace all `rdkafka` entries in your Cargo.toml:

```toml
[dependencies]
rdkafka = { version = "0.2.26", package = "madsim-rdkafka" }
```

## API Modification

This crate roughly follows the rdkafka API but is NOT exactly the same.

The following functions are modified to be `async`:

- `FromClientConfig::from_config`
- `FromClientConfigAndContext::from_config_and_context`
- `ClientConfig::create`
- `ClientConfig::create_with_context`
- `Client::fetch_metadata`
- `Client::fetch_watermarks`
- `Client::fetch_group_list`
- `Consumer::seek`
- `Consumer::seek_partitions`
- `Consumer::commit`
- `Consumer::commit_consumer_state`
- `Consumer::commit_message`
- `Consumer::committed`
- `Consumer::committed_offsets`
- `Consumer::offsets_for_timestamp`
- `Consumer::offsets_for_times`
- `Consumer::fetch_metadata`
- `Consumer::fetch_watermarks`
- `Consumer::fetch_group_list`
- `Producer::flush`
- `Producer::init_transactions`
- `Producer::send_offsets_to_transaction`
- `Producer::commit_transaction`
- `Producer::abort_transaction`
