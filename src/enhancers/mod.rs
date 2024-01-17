use smol_str::SmolStr;

mod actions;
mod frame;
mod grammar;
mod matchers;
mod rules;

use self::frame::Frame;
use self::rules::Rule;

#[derive(Debug, Clone, Default)]
pub struct ExceptionData {
    ty: Option<SmolStr>,
    value: Option<SmolStr>,
    mechanism: Option<SmolStr>,
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
    use std::time::{Duration, Instant};

    use serde_json::Value;

    use super::*;

    #[test]
    #[ignore = "needs to be run manually in release mode"]
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
        const ITERS: u64 = 1_000;
        for _ in 0..=ITERS {
            for frames in &mut stacktraces {
                enhancements.apply_modifications_to_frames(frames, &exception_data);
            }
        }

        let elapsed = instant.elapsed();
        let per_iter = Duration::from_nanos(elapsed.as_nanos() as u64 / ITERS);

        println!("Applied modifications in: {elapsed:?} / {per_iter:?} per application");
    }
}
