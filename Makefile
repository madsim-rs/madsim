.PHONY: build test sbuild stest

SIM_FLAGS := RUSTFLAGS="--cfg madsim" RUSTDOCFLAGS="--cfg madsim" CARGO_TARGET_DIR="target/sim"

build:
	cargo build

sbuild:
	$(SIM_FLAGS) cargo build

test:
	cargo test --workspace --exclude madsim-tokio-postgres

stest:
	$(SIM_FLAGS) cargo test --workspace --exclude madsim-tokio-postgres

clippy:
	cargo clippy

sclippy:
	$(SIM_FLAGS) cargo clippy

doc:
	$(SIM_FLAGS) cargo doc --no-deps
