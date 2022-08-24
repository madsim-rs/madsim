.PHONY: build test sbuild stest

SIM_FLAGS := RUSTFLAGS="--cfg madsim" RUSTDOCFLAGS="--cfg madsim" CARGO_TARGET_DIR="target/sim"

build:
	cargo build

sbuild:
	$(SIM_FLAGS) cargo build

test:
	cargo test

stest:
	$(SIM_FLAGS) cargo test

clippy:
	cargo clippy

sclippy:
	$(SIM_FLAGS) cargo clippy

doc:
	$(SIM_FLAGS) cargo doc --no-deps
