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

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use serde_json::Value;

    use super::*;

    #[test]
    fn apply_full() {
        let enhancers = std::fs::read_to_string("tests/fixtures/newstyle@2023-01-11.txt").unwrap();
        let enhancements = Enhancements::parse(&enhancers).unwrap();

        let event = std::fs::read_to_string("/Volumes/EncryptedScratchpad/event.json").unwrap();
        let event: serde_json::Value = serde_json::from_str(&event).unwrap();

        let platform = event
            .get("platform")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        let mut stacktraces = vec![];
        let mut collect_container = |c: &Value| {
            let empty = vec![];
            let frames = c
                .pointer("/stacktrace/frames")
                .and_then(|v| v.as_array())
                .unwrap_or(&empty);
            if !frames.is_empty() {
                let frames: Vec<_> = frames
                    .iter()
                    .map(|f| Frame::from_test(f, platform))
                    .collect();
                stacktraces.push(frames);
            }
        };

        for exc in event
            .pointer("/exception/values")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![])
        {
            collect_container(exc);
        }
        for thread in event
            .pointer("/threads/values")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![])
        {
            collect_container(thread);
        }

        let exception_data = ExceptionData {
            ty: Some(SmolStr::new("App Hanging")),
            value: Some(SmolStr::new("App hanging for at least 2000 ms.")),
            mechanism: Some(SmolStr::new("AppHang")),
        };

        //dbg!(&stacktraces);

        let instant = Instant::now();
        for _ in 0..=1_000 {
            for frames in &mut stacktraces {
                enhancements.apply_modifications_to_frames(frames, &exception_data);
            }
        }
        println!("Applied modifications in: {:?}", instant.elapsed());
    }
}
