use std::num::NonZeroUsize;

use super::rules::Rule;

pub trait Cache {
    fn get_or_try_insert<F>(&mut self, key: &str, f: F) -> anyhow::Result<Rule>
    where
        F: Fn(&str) -> anyhow::Result<Rule>;
}

// Rust, Y U NO impl this by default?
impl<C: Cache> Cache for &mut C {
    fn get_or_try_insert<F>(&mut self, key: &str, f: F) -> anyhow::Result<Rule>
    where
        F: Fn(&str) -> anyhow::Result<Rule>,
    {
        (*self).get_or_try_insert(key, f)
    }
}

pub struct NoopCache;
impl Cache for NoopCache {
    fn get_or_try_insert<F>(&mut self, key: &str, f: F) -> anyhow::Result<Rule>
    where
        F: Fn(&str) -> anyhow::Result<Rule>,
    {
        f(key)
    }
}

pub struct LruCache(lru::LruCache<Box<str>, Rule>);

impl LruCache {
    pub fn new(size: NonZeroUsize) -> Self {
        Self(lru::LruCache::new(size))
    }
}

impl Cache for LruCache {
    fn get_or_try_insert<F>(&mut self, key: &str, f: F) -> anyhow::Result<Rule>
    where
        F: Fn(&str) -> anyhow::Result<Rule>,
    {
        if let Some(rule) = self.0.get(key) {
            return Ok(rule.clone());
        }

        let rule = f(key)?;
        self.0.put(key.into(), rule.clone());
        Ok(rule)
    }
}
