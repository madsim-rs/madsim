# madsim-s3-client

The `aws-sdk-s3` simulator on madsim. Mirrors [aws-sdk-s3 v0.28.0](https://docs.rs/aws-sdk-s3/0.28.0/aws_sdk_s3/index.html).

> If it looks like s3, acts like s3, and is used like s3, then it probably is s3.

## Usage

Replace all `aws-sdk-s3` entries in your Cargo.toml:

```toml
[dependencies]
aws-sdk-s3 = { version = "0.2", package = "madsim-aws-sdk-s3" }
```
