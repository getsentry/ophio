//! CODEOWNERS path pattern matching.

use std::num::NonZeroUsize;
use std::sync::{LazyLock, Mutex};

use lru::LruCache;
use regex::bytes::Regex;

static CODEOWNERS_CACHE: LazyLock<Mutex<LruCache<String, Regex>>> =
    LazyLock::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(500).unwrap())));

fn translate_codeowners_pattern(pattern: &str) -> Option<Regex> {
    let mut regex = String::new();

    if pattern.starts_with('\\') {
        return Regex::new(r"\\(?:\z|/)").ok();
    }

    let anchored = pattern
        .find('/')
        .is_some_and(|position| position != pattern.len() - 1);

    if anchored {
        regex += r"\A";
    } else {
        regex += r"(?:\A|/)";
    }

    let matches_dir = pattern.ends_with('/');
    let mut pattern = pattern;
    if matches_dir {
        pattern = pattern.trim_end_matches('/');
    }

    let trailing_slash_star = pattern.len() > 1 && pattern.ends_with("/*");
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let mut num_to_skip = None;

    for (index, &character) in pattern_chars.iter().enumerate() {
        if index == 0 && anchored && pattern.starts_with('/') {
            regex += r"/?";
            continue;
        }

        if let Some(skip_amount) = num_to_skip {
            num_to_skip = Some(skip_amount - 1);
            if num_to_skip > Some(0) {
                continue;
            }
        }

        if character == '*' {
            if pattern_chars.get(index + 1) == Some(&'*') {
                let left_anchored = index == 0;
                let leading_slash = index > 0 && pattern_chars.get(index - 1) == Some(&'/');
                let right_anchored = index + 2 == pattern.len();
                let trailing_slash = pattern_chars.get(index + 2) == Some(&'/');

                if (left_anchored || leading_slash) && (right_anchored || trailing_slash) {
                    if trailing_slash {
                        regex += "(?:.*/)?";
                        num_to_skip = Some(3);
                    } else {
                        regex += ".*";
                        num_to_skip = Some(2);
                    }
                    continue;
                }
            }
            regex += r"[^/]*";
        } else if character == '?' {
            regex += r"[^/]";
        } else {
            regex += &regex::escape(character.to_string().as_str());
        }
    }

    if matches_dir {
        regex += "/";
    } else if trailing_slash_star {
        regex += r"\z";
    } else {
        regex += r"(?:\z|/)";
    }
    Regex::new(&regex).ok()
}

/// Returns `true` if the CODEOWNERS pattern matches the path, `false` otherwise.
///
/// Compiled patterns are cached. Matching is case sensitive and follows CODEOWNERS semantics,
/// rather than standard glob semantics.
pub fn is_codeowners_path_match(value: &[u8], pattern: &str) -> bool {
    let mut cache = CODEOWNERS_CACHE.lock().unwrap();

    if let Some(pattern) = cache.get(pattern) {
        pattern.is_match(value)
    } else if let Some(compiled_pattern) = translate_codeowners_pattern(pattern) {
        let result = compiled_pattern.is_match(value);
        cache.put(pattern.to_owned(), compiled_pattern);
        result
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codeowners_patterns() {
        let cases = [
            ("file.txt", "*.txt", true),
            ("file.txt/", "*.txt", true),
            ("dir/file.txt", "*.txt", true),
            ("/dir/file.txt", "/dir/*.txt", true),
            ("dir/file.txt", "/dir/*.txt", true),
            ("/dir/subdir/file.txt", "/dir/*.txt", false),
            ("apps/file.txt", "apps/", true),
            ("/apps/file.txt", "apps/", true),
            ("/dir/apps/file.txt", "apps/", true),
            ("/dir/subdir/apps/file.txt", "apps/", true),
            ("docs/getting-started.md", "docs/*", true),
            ("docs/build-app/troubleshooting.md", "docs/*", false),
            (
                "something/docs/build-app/troubleshooting.md",
                "docs/*",
                false,
            ),
            ("first/docs/troubleshooting.md", "*/docs/", true),
            ("docs/getting-started.md", "*/docs/", false),
            ("first/second/docs/troubleshooting.md", "*/docs/", false),
            (
                "docs/first/something/troubleshooting.md",
                "docs/*/something/",
                true,
            ),
            (
                "something/docs/first/something/troubleshooting.md",
                "docs/*/something/",
                false,
            ),
            (
                "docs/first/second/something/getting-started.md",
                "docs/*/something/",
                false,
            ),
            ("/docs/file.txt", "/docs/", true),
            ("/docs/subdir/file.txt", "/docs/", true),
            ("docs/subdir/file.txt", "/docs/", true),
            ("app/docs/file.txt", "/docs/", false),
            ("red/orange/yellow/green/file.py", "/red/**/file.py", true),
            ("/red/orange/file.py", "/red/**/file.py", true),
            ("red/file.py", "/red/**/file.py", true),
            ("yellow/file.py", "/red/**/file.py", false),
            ("red/orange/yellow/green/file.py", "red/**/file.py", true),
            ("red/file.py", "red/**/file.py", true),
            ("something/red/file.py", "red/**/file.py", false),
            ("red/orange/yellow/file.py", "**/yellow/file.py", true),
            ("yellow/file.py", "**/yellow/file.py", true),
            ("/docs/file.py", "**/yellow/file.py", false),
            ("red/orange/yellow/file.py", "/red/orange/**", true),
            ("red/orange/file.py", "/red/orange/**", true),
            ("blue/red/orange/file.py", "/red/orange/**", false),
            ("/docs/subdir/file.css", "/**/*.css", true),
            ("file.css", "/**/*.css", true),
            ("/docs/file.txt", "/**/*.css", false),
            ("foo.py", "/**/foo.py", true),
            ("dir/foo.py", "/**/foo.py", true),
            ("dir/subdir/foo.py", "/**/foo.py", true),
            ("not_foo.py", "/**/foo.py", false),
            ("dir/not_foo.py", "/**/foo.py", false),
        ];

        for (value, pattern, expected) in cases {
            assert_eq!(
                is_codeowners_path_match(value.as_bytes(), pattern),
                expected,
                "pattern {pattern:?}, value {value:?}",
            );
        }
    }
}
