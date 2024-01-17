use std::num::NonZeroUsize;

use smol_str::SmolStr;

mod actions;
mod cache;
mod frame;
mod grammar;
mod matchers;
mod rules;

pub use self::frame::Frame;
use self::rules::Rule;
pub use cache::*;

#[derive(Debug, Clone, Default)]
pub struct ExceptionData {
    pub ty: Option<SmolStr>,
    pub value: Option<SmolStr>,
    pub mechanism: Option<SmolStr>,
}

#[derive(Debug)]
pub struct Enhancements {
    all_rules: Vec<Rule>,
    modifier_rules: Vec<Rule>,
}

impl Enhancements {
    pub fn parse(input: &str, mut cache: impl Cache) -> anyhow::Result<Self> {
        let mut all_rules = vec![];

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let rule = cache.get_or_try_insert(line, grammar::parse_rule)?;
            all_rules.push(rule);
        }

        let modifier_rules = all_rules
            .iter()
            .filter(|r| r.has_modifier_action())
            .cloned()
            .collect();

        Ok(Enhancements {
            all_rules,
            modifier_rules,
        })
    }

    pub fn apply_modifications_to_frames(
        &self,
        frames: &mut [Frame],
        exception_data: &ExceptionData,
    ) {
        for rule in &self.modifier_rules {
            if !rule.matches_exception(exception_data) {
                continue;
            }

            for idx in 0..frames.len() {
                if rule.matches_frame(frames, idx) {
                    rule.apply_modifications_to_frame(frames, idx);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_enhancers() {
        let enhancers = std::fs::read_to_string("tests/fixtures/newstyle@2023-01-11.txt").unwrap();
        Enhancements::parse(&enhancers, NoopCache).unwrap();
    }
}
