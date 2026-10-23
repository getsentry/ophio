.PHONY: develop test install-py-dev build

develop \
install-js-dev \
install-rs-dev :
	@make devenv-sync

# This is to ensure devenv sync's only called once if the above
# macros are combined e.g. `make install-rs-dev install-py-dev`
.PHONY: devenv-sync
devenv-sync:
	devenv sync


test:
	OPHIO_SETTINGS=test .venv/bin/pytest -vv tests -v -m "not ci_only"

tests: test

# install-rs-dev/install-py-dev mimick sentry's naming conventions

test-rust:
	cargo test --workspace
.PHONY: test-rust

lint-rust:
	cargo clippy --workspace --all-targets --no-deps -- -D warnings
.PHONY: lint-rust

format-rust:
	cargo +stable fmt --all
.PHONY: format-rust

format-rust-ci:
	cargo +stable fmt --all --check
.PHONY: format-rust-ci

build:
	maturin develop
.PHONY: build
