# Changelog

## 0.1.4

### Various fixes & improvements

- fix craft auto changelogs (d95103ea) by @Swatinem
- Ignore Broken Glob matchers (#38) by @Swatinem
- Add a LICENSE (#37) by @Swatinem
- Downgrade Python requirement (#36) by @Swatinem

## 0.1.1

Create `sentry_ophio` as a generic dumping ground for Rust code with Python
bindings for usage within Sentry.

So far this has:

- The `proguard` bindings that were previously living in `symbolic`
- `enhancers` which implements the grouping enhancers code, including parsing
  the enhancement rules, and applying those to stack traces.

