.PHONY: build test sbuild stest

build:
	cargo build

sbuild:
	RUSTFLAGS="--cfg madsim" \
	RUSTDOCFLAGS="--cfg madsim" \
	CARGO_TARGET_DIR="target/sim" \
	cargo build

test:
	cargo test

stest:
	RUSTFLAGS="--cfg madsim" \
	RUSTDOCFLAGS="--cfg madsim" \
	CARGO_TARGET_DIR="target/sim" \
	cargo test
