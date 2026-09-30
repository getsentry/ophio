//! Binary glob pattern matching.

use std::borrow::Cow;
use std::num::NonZeroUsize;
use std::sync::{LazyLock, Mutex, PoisonError};

use globset::GlobBuilder;
use lru::LruCache;
use regex::bytes::{Regex, RegexBuilder};

static GLOB_CACHE: LazyLock<Mutex<LruCache<(GlobOptions, String), Regex>>> =
    LazyLock::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(500).unwrap())));

/// Controls the options of the globber.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct GlobOptions {
    /// When enabled `**` matches over path separators and `*` does not.
    pub double_star: bool,
    /// Enables case insensitive path matching.
    pub case_insensitive: bool,
    /// Enables path normalization.
    pub path_normalize: bool,
    /// Allows newlines.
    pub allow_newline: bool,
}

fn translate_pattern(pattern: &str, options: GlobOptions) -> Option<Regex> {
    let mut builder = GlobBuilder::new(pattern);
    builder.case_insensitive(options.case_insensitive);
    builder.literal_separator(options.double_star);
    let glob = builder.build().ok()?;
    RegexBuilder::new(glob.regex())
        .dot_matches_new_line(options.allow_newline)
        .build()
        .ok()
}

/// Returns `true` if the glob matches the bytes, `false` otherwise.
///
/// Invalid patterns return `false`. Compiled patterns are cached together with their options.
pub fn is_glob_match(value: &[u8], pattern: &str, options: GlobOptions) -> bool {
    let (value, pattern) = if options.path_normalize {
        (
            Cow::Owned(
                value
                    .iter()
                    .map(|&byte| if byte == b'\\' { b'/' } else { byte })
                    .collect(),
            ),
            pattern.replace('\\', "/"),
        )
    } else {
        (Cow::Borrowed(value), pattern.to_owned())
    };
    let key = (options, pattern);
    let mut cache = GLOB_CACHE.lock().unwrap_or_else(PoisonError::into_inner);

    if let Some(pattern) = cache.get(&key) {
        pattern.is_match(&value)
    } else if let Some(pattern) = translate_pattern(&key.1, options) {
        let result = pattern.is_match(&value);
        cache.put(key, pattern);
        result
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_globs() {
        macro_rules! test_glob {
            ($value:expr, $pattern:expr, $expected:expr, {$($field:ident: $value_option:expr),*}) => {{
                let options = GlobOptions { $($field: $value_option,)* ..Default::default() };
                assert_eq!(
                    is_glob_match($value.as_bytes(), $pattern, options),
                    $expected,
                    "pattern {:?}, value {:?}, options {:?}",
                    $pattern,
                    $value,
                    options,
                );
            }}
        }

        test_glob!("hello.py", "*.py", true, {});
        test_glob!("hello.py", "*.js", false, {});
        test_glob!("foo/hello.py", "*.py", true, {});
        test_glob!("foo/hello.py", "*.py", false, {double_star: true});
        test_glob!("foo/hello.py", "**/*.py", true, {double_star: true});
        test_glob!("foo/hello.PY", "**/*.py", false, {double_star: true});
        test_glob!("foo/hello.PY", "**/*.py", true, {double_star: true, case_insensitive: true});
        test_glob!("foo\\hello\\bar.PY", "foo/**/*.py", false, {double_star: true, case_insensitive: true});
        test_glob!("foo\\hello\\bar.PY", "foo/**/*.py", true, {double_star: true, case_insensitive: true, path_normalize: true});
        test_glob!("foo\\hello\\bar.PY", "foo\\**\\*.py", true, {double_star: true, case_insensitive: true, path_normalize: true});
        test_glob!("foo\nbar", "foo*", false, {});
        test_glob!("foo\nbar", "foo*", true, {allow_newline: true});
        test_glob!("1.18.4.2153-2aa83397b", "1.18.[0-4].*", true, {});
        test_glob!("hello.py", "[", false, {});
        test_glob!("hello.py", "{", false, {});
        test_glob!("", "*", true, {});
        test_glob!("", "", true, {});
        test_glob!("hello.py", "", false, {});
    }

    #[test]
    fn test_binary_value() {
        assert!(is_glob_match(b"\xff.py", "*.py", GlobOptions::default()));
        assert!(is_glob_match(b"hello\0.py", "*.py", GlobOptions::default()));
    }

    #[test]
    fn test_long_value() {
        let mut value = "x".repeat(1_000_000);
        value.push_str(".PY");
        let options = GlobOptions {
            double_star: true,
            case_insensitive: true,
            path_normalize: true,
            ..Default::default()
        };
        assert!(is_glob_match(
            value.as_bytes(),
            "*************************.py",
            options,
        ));
        assert!(!is_glob_match(
            value.as_bytes(),
            "*************************.js",
            options,
        ));
    }
}
