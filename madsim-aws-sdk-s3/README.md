# madsim-s3-client

The `aws-sdk-s3` simulator on madsim. Mirrors [aws-sdk-s3 v1.2.0](https://docs.rs/aws-sdk-s3/1.2.0/aws_sdk_s3/index.html).

> If it looks like s3, acts like s3, and is used like s3, then it probably is s3.

## Usage

Replace all `aws-sdk-s3` entries in your Cargo.toml:

```toml
[dependencies]
aws-sdk-s3 = { version = "0.5", package = "madsim-aws-sdk-s3" }
```

By default, this crate enables `default-https-client`, `http-1x`, `rt-tokio`, and
`sigv4a`.

Its public feature names follow the upstream `aws-sdk-s3` crate. In particular,
`default-https-client` and `rustls` keep the same meaning as upstream, while this
crate's default feature set prefers `default-https-client`.
