use lru::LruCache;
use regex::bytes::Regex;

use super::rules::Rule;

/// An LRU cache for parsing [`Rule`]s.
#[derive(Debug, Default)]
pub struct Cache {
    rules: Option<LruCache<Box<str>, Rule>>,
    regex: Option<LruCache<Box<str>, Regex>>,
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

    /// Gets the rule for the string `key` from the cache or computes and inserts
    /// it using `f` if it is not present.
    pub fn get_or_try_insert_rule<F>(&mut self, key: &str, f: F) -> anyhow::Result<Rule>
    where
        F: Fn(&str) -> anyhow::Result<Rule>,
    {
        match self.rules.as_mut() {
            Some(cache) => {
                if let Some(rule) = cache.get(key) {
                    return Ok(rule.clone());
                }

                let rule = f(key)?;
                cache.put(key.into(), rule.clone());
                Ok(rule)
            }
            None => f(key),
        }
    }

    pub fn get_or_try_insert_regex<F>(&mut self, key: &str, f: F) -> anyhow::Result<Regex>
    where
        F: Fn(&str) -> anyhow::Result<Regex>,
    {
        match self.regex.as_mut() {
            Some(cache) => {
                if let Some(regex) = cache.get(key) {
                    return Ok(regex.clone());
                }

                let regex = f(key)?;
                cache.put(key.into(), regex.clone());
                Ok(regex)
            }
            None => f(key),
        }
    }
}
