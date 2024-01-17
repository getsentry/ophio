# Ophio

An attempt at creating some generic Rust bindings to use within Sentry.

See also [RFC119](https://github.com/getsentry/rfcs/pull/119) for more details.

## How To use?

For now, do a `maturin build`, and then in your `sentry` install:

> pip install -e ../ophio

Now you can `import sentry_ophio` and `from sentry_ophio import XXX` within `sentry`.

## Whats with the name?

> Ophiogomphus rupinsulensis, the rusty snaketail, is a species of clubtail in the family of dragonflies known as Gomphidae.

[Wikipedia](https://en.wikipedia.org/wiki/Ophiogomphus_rupinsulensis)
