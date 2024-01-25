//! Caching logic to improve the performance of creating grouping enhancements.

use std::sync::Arc;

use globset::GlobBuilder;
use lru::LruCache;
use regex::bytes::{Regex, RegexBuilder};
use smol_str::SmolStr;

use super::{grammar::parse_rule, rules::Rule};

/// An LRU cache for memoizing the construction of [`Rules`](Rule) and [`Regexes`](Regex).
#[derive(Debug, Default)]
pub struct Cache {
    rules: Option<LruCache<SmolStr, Rule>>,
    regex: Option<LruCache<(SmolStr, bool), Arc<Regex>>>,
}

impl Cache {
    /// Creates a new cache with the given size.
    ///
    /// If `size` is 0, no caching will be performed.
    pub fn new(size: usize) -> Self {
        let rules = size.try_into().ok().map(LruCache::new);
        let regex = size.try_into().ok().map(LruCache::new);
        Self { rules, regex }
    }

    /// Gets the rule for the string `key` from the cache or parses and inserts
    /// it using `parse_rule` if it is not present.
    pub fn get_or_try_insert_rule(&mut self, key: &str) -> anyhow::Result<Rule> {
        match self.rules.as_mut() {
            Some(cache) => {
                if let Some(rule) = cache.get(key) {
                    return Ok(rule.clone());
                }

                let rule = parse_rule(key)?;
                cache.put(key.into(), rule.clone());
                Ok(rule)
            }
            None => parse_rule(key),
        }
    }

    /// Gets the regex for the string `key` and the boolean `is_path` from the cache or computes and inserts
    /// it using `translate_pattern` if it is not present.
    pub fn get_or_try_insert_regex(
        &mut self,
        key: &str,
        is_path: bool,
    ) -> anyhow::Result<Arc<Regex>> {
        match self.regex.as_mut() {
            Some(cache) => {
                let key = (key.into(), is_path);
                if let Some(regex) = cache.get(&key) {
                    return Ok(regex.clone());
                }

                let regex = translate_pattern(&key.0, key.1).map(Arc::new)?;
                cache.put(key, regex.clone());
                Ok(regex)
            }
            None => translate_pattern(key, is_path).map(Arc::new),
        }
    }
}

/// Translates a glob pattern to a regex.
///
/// If `is_path_matcher` is true, backslashes in the pattern will be normalized
/// to slashes and `*` won't match path separators (i.e. `**` must be used to match
/// multiple path segments).
fn translate_pattern(pat: &str, is_path_matcher: bool) -> anyhow::Result<Regex> {
    let pat = if is_path_matcher {
        pat.replace('\\', "/")
    } else {
        pat.into()
    };
    let mut builder = GlobBuilder::new(&pat);
    builder.literal_separator(is_path_matcher);
    builder.case_insensitive(is_path_matcher);
    let glob = builder.build()?;
    Ok(RegexBuilder::new(glob.regex()).build()?)
}
