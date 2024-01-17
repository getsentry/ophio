use smol_str::SmolStr;

mod actions;
mod frame;
mod grammar;
mod matchers;
mod rules;

pub use self::frame::Frame;
use self::rules::Rule;

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
    pub fn parse(input: &str) -> anyhow::Result<Self> {
        grammar::parse_enhancers(input)
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
        Enhancements::parse(&enhancers).unwrap();
    }
}
