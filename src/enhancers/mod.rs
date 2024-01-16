use smol_str::SmolStr;

mod actions;
mod frame;
mod grammar;
mod matchers;
mod rules;

use self::frame::Frame;
use self::grammar::rule;
use self::rules::Rule;

#[derive(Debug, Clone, Default)]
pub struct ExceptionData {
    ty: Option<SmolStr>,
    value: Option<SmolStr>,
    mechanism: Option<SmolStr>,
}

#[derive(Debug)]
pub struct Enhancements {
    rules: Vec<Rule>,
}
impl Enhancements {
    pub fn parse(input: &str) -> anyhow::Result<Self> {
        let mut rules = vec![];

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // TODO: caching if it makes a difference

            let raw_rule = rule(line)?;
            let rule = Rule::from_raw(raw_rule)?;
            rules.push(rule);
        }

        Ok(Self { rules })
    }

    pub fn apply_modifications_to_frames(
        &self,
        frames: &mut [Frame],
        exception_data: &ExceptionData,
    ) {
        for rule in &self.rules {
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
