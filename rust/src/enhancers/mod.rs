use smol_str::SmolStr;

mod actions;
mod cache;
mod frame;
mod grammar;
mod matchers;
mod rules;

pub use self::frame::{Frame, StringField};
pub use self::rules::Rule;
pub use cache::*;

#[derive(Debug, Clone, Default)]
pub struct ExceptionData {
    pub ty: Option<SmolStr>,
    pub value: Option<SmolStr>,
    pub mechanism: Option<SmolStr>,
}

#[derive(Debug, Default)]
pub struct Enhancements {
    pub(crate) all_rules: Vec<Rule>,
    modifier_rules: Vec<Rule>,
}

impl Enhancements {
    pub fn parse(input: &str, cache: &mut Cache) -> anyhow::Result<Self> {
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
        let mut matching_frames = Vec::with_capacity(frames.len());
        for rule in &self.modifier_rules {
            if !rule.matches_exception(exception_data) {
                continue;
            }

            // first, for each frame check if the rule matches
            matching_frames
                .extend((0..frames.len()).filter(|idx| rule.matches_frame(frames, *idx)));

            // then in a second pass, apply the actions to all matching frames
            for idx in matching_frames.drain(..) {
                rule.apply_modifications_to_frame(frames, idx);
            }
        }
    }

    pub fn rules(&self) -> impl Iterator<Item = &Rule> {
        self.all_rules.iter()
    }

    pub fn extend_from(&mut self, other: &Enhancements) {
        self.extend(other.rules().cloned())
    }
}

impl Extend<Rule> for Enhancements {
    fn extend<T: IntoIterator<Item = Rule>>(&mut self, iter: T) {
        for rule in iter.into_iter() {
            if rule.has_modifier_action() {
                self.modifier_rules.push(rule.clone());
            }

            self.all_rules.push(rule);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_enhancers() {
        let enhancers =
            std::fs::read_to_string("../tests/fixtures/newstyle@2023-01-11.txt").unwrap();
        Enhancements::parse(&enhancers, &mut Cache::default()).unwrap();
    }
}
