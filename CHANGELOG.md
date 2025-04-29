# Changelog

## 1.1.0

### Various fixes & improvements

- allow enhancement versions 2 or higher (#73) by @lobsterkatie
- meta: processing -> ingest codeowners (#75) by @Dav1dde
- Bump pyo3 from 0.22.4 to 0.24.1 (#72) by @dependabot
- Bump pyo3 from 0.22.1 to 0.22.4 (#70) by @dependabot
- ci: update artifact actions to v4 (#69) by @joshuarli

## 1.0.0

### Various fixes & improvements

- 3.10.11 (#65) by @armenzg
- remove wityh (#65) by @armenzg
- Cargo toml (#65) by @armenzg
- Do not run on push for pull requests (#68) by @armenzg
- Add python-version to tests (#66) by @armenzg
- 3.10.15 (#66) by @armenzg
- Partially revert "Update to Python 3.11 and `cargo update` (#56)" (#66) by @armenzg
- Build wheels with Python 3.10 (#65) by @armenzg
- No need for if conditionals (#67) by @armenzg
- Run release builds on PRs (#67) by @armenzg

## 0.2.9

### Various fixes & improvements

- Lower the minimum Python requirement (#64) by @armenzg

## 0.2.8

### Various fixes & improvements

- Remove `is_sentinel/prefix_frame` from Enhancers (#63) by @Swatinem
- Remove more unused dependencies (#61) by @Swatinem
- Update pyo3 and clean up dependencies (#60) by @Swatinem
- Remove Proguard bindings (#59) by @Swatinem
- Remove all the `ketama`-related code (#55) by @Swatinem
- Update to Python 3.11 and `cargo update` (#56) by @Swatinem

## 0.2.7

### Various fixes & improvements

- Implement a crc32/Ketama-based consistent hashing scheme (#52) by @Swatinem

## 0.2.6

### Various fixes & improvements

- Bump black from 23.12.0 to 24.3.0 (#51) by @dependabot
- Return prettier parse errors (#50) by @Swatinem

## 0.2.5

### Various fixes & improvements

- Fully replicate `in_app` hint logic (#49) by @Swatinem

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

