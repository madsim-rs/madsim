# madsim-norandom

An intercept library for libc `getrandom` and `getentropy` function.

## Usage

Build:

```sh
cargo build
```

Run:

```sh
# Linux
LD_PRELOAD=../target/debug/libmadsim_norandom.so ./app
# macOS
DYLD_INSERT_LIBRARIES=../target/debug/libmadsim_norandom.dylib ./app
```

You can also set the `MADSIM_TEST_SEED` environment variable to control the random number generated:

```
MADSIM_TEST_SEED=1 LD_PRELOAD=../target/debug/libmadsim_norandom.so ./app
```

By default the seed is set to 0.
