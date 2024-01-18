use lru::LruCache;

use super::rules::Rule;

/// An LRU cache for parsing [`Rule`]s.
#[derive(Debug, Default)]
pub struct Cache(Option<LruCache<Box<str>, Rule>>);

impl Cache {
    /// Creates a new cache with the given size.
    ///
    /// If `size` is 0, no caching will be performed.
    pub fn new(size: usize) -> Self {
        Self(size.try_into().ok().map(|n| LruCache::new(n)))
    }

    /// Gets the rule for the string `key` from the cache or computes and inserts
    /// it using `f` if it is not present.
    pub fn get_or_try_insert<F>(&mut self, key: &str, f: F) -> anyhow::Result<Rule>
    where
        F: Fn(&str) -> anyhow::Result<Rule>,
    {
        match self.0.as_mut() {
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
}
