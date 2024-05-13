# madsim-rdkafka

[![Crate](https://img.shields.io/crates/v/madsim-rdkafka.svg)](https://crates.io/crates/madsim-rdkafka)
[![Docs](https://docs.rs/madsim-rdkafka/badge.svg)](https://docs.rs/madsim-rdkafka)

The `rdkafka` simulator on madsim. Mirrors [rdkafka v0.34.0](https://docs.rs/rdkafka/0.34.0/rdkafka/index.html) and librdkafka 2.3.0.

## Usage

Replace all `rdkafka` entries in your Cargo.toml:

```toml
[dependencies]
rdkafka = { version = "0.4", package = "madsim-rdkafka" }
```

## API Modification

This crate roughly follows the rdkafka API but is NOT exactly the same.

The following functions are modified to be `async`:

- `FromClientConfig::from_config`
- `FromClientConfigAndContext::from_config_and_context`
- `ClientConfig::create`
- `ClientConfig::create_with_context`
- `Client::fetch_metadata`[^1]
- `Client::fetch_watermarks`[^1]
- `Client::fetch_group_list`[^1]
- `Consumer::seek`
- `Consumer::seek_partitions`
- `Consumer::commit`
- `Consumer::commit_consumer_state`
- `Consumer::commit_message`
- `Consumer::committed`
- `Consumer::committed_offsets`
- `Consumer::offsets_for_timestamp`
- `Consumer::offsets_for_times`[^1]
- `Consumer::fetch_metadata`[^1]
- `Consumer::fetch_watermarks`[^1]
- `Consumer::fetch_group_list`[^1]
- `Producer::flush`
- `Producer::init_transactions`
- `Producer::send_offsets_to_transaction`
- `Producer::commit_transaction`
- `Producer::abort_transaction`

[^1]: wrapped in `tokio::task::spawn_blocking`

The associated constant `ClientContext::ENABLE_REFRESH_OAUTH_TOKEN` is changed to a function in order to make the trait object-safe.

## DNS Resolution

This crate has cherry-picked [a commit] from Materialize to support rewriting broker addresses.

[a commit]: https://github.com/MaterializeInc/rust-rdkafka/commit/8ea07c4d2b96636ff093e670bc921892aee0d56a

A new method is added to `ClientContext`:

- `ClientContext::rewrite_broker_addr`
