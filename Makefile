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
	cargo clippy --all-targets

sclippy:
	$(SIM_FLAGS) cargo clippy --all-targets

doc:
	$(SIM_FLAGS) cargo doc --no-deps
