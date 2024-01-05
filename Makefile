.PHONY: build test sbuild stest

SIM_FLAGS := RUSTFLAGS="--cfg madsim" RUSTDOCFLAGS="--cfg madsim" CARGO_TARGET_DIR="target/sim"

build:
	cargo build

sbuild:
	$(SIM_FLAGS) cargo build

test:
	cargo test

stest:
	$(SIM_FLAGS) cargo test --all-features

check:
	cargo check --all-targets

scheck:
	$(SIM_FLAGS) cargo check --all-features --all-targets

clippy:
	cargo clippy --all-targets

sclippy:
	$(SIM_FLAGS) cargo clippy --all-features --all-targets

doc:
	$(SIM_FLAGS) cargo doc --no-deps
