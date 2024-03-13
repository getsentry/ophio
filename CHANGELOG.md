# Changelog

## 0.2.3

### Various fixes & improvements

- fix(bindings): Add type info for Component.__new__ (#48) by @loewenheim
- Fix typing for pyi `ExceptionData` (#47) by @Swatinem

## 0.2.0

### Various fixes & improvements

- Add the complete `assemble_stacktrace_component` logic (#46) by @Swatinem
- Switch to new PyO3 `Bound` API (#45) by @Swatinem
- Document matching behavior without any matcher (#44) by @Swatinem
- Implement `update_frame_components_contributions` (#42) by @Swatinem
- Do a `cargo update` (#43) by @Swatinem
- ref(enhancers): Replace nom parser with handwritten recursive descent (#40) by @loewenheim

## 0.1.5

### Various fixes & improvements

- Silently accept invalid `app` matcher (#41) by @Swatinem

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

