# Changelog

## 0.1.1

Create `sentry_ophio` as a generic dumping ground for Rust code with Python
bindings for usage within Sentry.

So far this has:

- The `proguard` bindings that were previously living in `symbolic`
- `enhancers` which implements the grouping enhancers code, including parsing
  the enhancement rules, and applying those to stack traces.

