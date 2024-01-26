# Ophio

An attempt at creating some generic Rust bindings to use within Sentry.

See also [RFC119](https://github.com/getsentry/rfcs/pull/119) for more details.

## How to write Bindings

- If you want to expose functionality from an existing crate:
  - Write bindings for it in `bindings/`.
- Otherwise, if this is purpose built code not intended for usage outside of Sentry:
  - Write (and test) straight Rust code in the `rust/` crate.
  - Write bindings for it in `bindings/`.
- Re-export all the code from `bindings/` within `python/sentry_ophio`.
- If necessary, write custom `pyi` typings in `python/sentry_ophio`.
- Write some `pytest` tests the `tests/`.
- After publishing, you import the code in the main `sentry` repo from the `sentry_ophio` package.

For more details, consult the [Repo Structure](#repo-structure) section.

## Publishing

The release / publish workflow is fully set up in the repo.

We do not maintain any kind of `semver` guarantees whatsoever.
For this reason, one can just increment the minor version for every new release / publish.

- Manually trigger the [`Release workflow`](actions/workflows/release.yml)
- This should ideally auto-approve the publish, publish to `pypi` and then uplift that to our
  internal `pypi` mirror as well.
- If the publish fails, which it unfortunately does way too often, search for a soundproof booth
  and scream from the top of your lungs, then go to `#z-vent` and complain loudly about
  the broken publishes and that we can’t have nice things.

## Repo Structure

This repo contains different pieces of code:

### `rust/`

This directory / crate contains purpose built Rust code to be used within Sentry.

The main Rust crate can contain unit tests, integration tests as well as benchmarks.

It is _not_ being published to the `crates.io` index, as it is not intended for outside consumption.

### `bindings/`

This directory / crate contains the `PyO3`-based Python bindings.
It contains and exports the main classes / functions to be used from Python.

Everything that is intended for export needs to be added to the main `#[pymodule]` within `lib.rs`.
All the exports are defined within a flat namespace.

The bindings crate can also export functionality to Python that is not defined in the `rust/` crate,
but rather comes from any public `crates.io` crate, no matter if it is maintained by Sentry,
or a third party.

### `python/sentry_ophio/`

This directory contains a tiny Python shim around the `bindings/`.

It houses Python type annotations in the form of `.pyi` files, as well as re-exporting everything
from the `bindings/` crate as well organized Python packages instead of a flat namespace.

If necessary, more glue code can be added here if it makes more sense to write it in Python rather
than in Rust.

All the code written in the `sentry_ophio` package will be published to `pypi` and will be
available within Sentry.

Using it is as easy as `from sentry_ophio.PACKAGE import STUFF`.

### `tests/`

This directory contains a `pytest` test suite containing tests for all the code exported through
the `python/sentry_ophio/` package.

## Local development

Make sure you have an up-to-date Rust compiler, which means running `rustup update`
at least once every 6 weeks.

Otherwise, `direnv` should set up the local dev environment automatically, and make a local
build of the bindings available directly for usage in Python and `pytest`.

To bind the local development version to the main Sentry repo,
you can run the following within the `sentry` workspace:

> pip install -e ../ophio

Afterwards, `sentry` will automatically pick up any changes within this workspace.
Then, run `maturin develop` on every change to rebuild the bindings.

## Whats with the name?

> Ophiogomphus rupinsulensis, the rusty snaketail, is a species of clubtail in the family of dragonflies known as Gomphidae.

[Wikipedia](https://en.wikipedia.org/wiki/Ophiogomphus_rupinsulensis)
