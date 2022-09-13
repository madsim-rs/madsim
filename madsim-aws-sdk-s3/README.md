# madsim-s3-client

The `aws-sdk-s3` simulator on madsim.

> If it looks like etcd-client, acts like etcd-client, and is used like etcd-client, then it probably is etcd-client.

## Usage

Replace all `aws-sdk-s3` entries in your Cargo.toml:

```toml
[dependencies]
aws-sdk-s3 = { version = "0.2", package = "madsim-aws-sdk-s3" }
```
