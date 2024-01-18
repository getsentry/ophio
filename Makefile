.PHONY: develop test install-python-dependencies install-py-dev

reset-python:
	pre-commit clean
	rm -rf .venv
.PHONY: reset-python

develop: install-python-dependencies install-rs-dev

test:
	OPHIO_SETTINGS=test pytest -vv tests -v -m "not ci_only"

tests: test

# install-rs-dev/install-py-dev mimick sentry's naming conventions

INDEX_URL=--index-url https://pypi.devinfra.sentry.io/simple

install-python-dependencies:
	pip uninstall -qqy uwsgi  # pip doesn't do well with swapping drop-ins
	pip install $(INDEX_URL) -r requirements-build.txt
	pip install $(INDEX_URL) -r requirements.txt
	pip install $(INDEX_URL) -e .
.PHONY: install-python-dependencies

install-rs-dev:
	@which cargo || (echo "!!! You need an installation of Rust in order to develop ophio. Go to https://rustup.rs to get one." && exit 1)
	. scripts/rust-envvars &&  maturin develop
.PHONY: install-rs-dev

install-py-dev: install-python-dependencies
.PHONY: install-py-dev

test-rust:
	. scripts/rust-envvars && \
		cargo test --workspace
.PHONY: test-rust

lint-rust:
	. scripts/rust-envvars && \
		cargo clippy --workspace --all-targets --no-deps -- -D warnings
.PHONY: lint-rust

format-rust:
	. scripts/rust-envvars && \
		cargo +stable fmt --all
.PHONY: format-rust

format-rust-ci:
	. scripts/rust-envvars && \
		cargo +stable fmt --all --check
.PHONY: format-rust-ci
