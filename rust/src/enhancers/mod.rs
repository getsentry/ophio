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

#[derive(Debug)]
pub struct Enhancements {
    pub(crate) all_rules: Vec<Rule>,
    modifier_rules: Vec<Rule>,
    updater_rules: Vec<Rule>,
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

        let updater_rules = all_rules
            .iter()
            .filter(|r| r.has_updater_action())
            .cloned()
            .collect();

        Ok(Enhancements {
            all_rules,
            modifier_rules,
            updater_rules,
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

    pub fn update_frame_components_contributions(
        &self,
        components: &mut [Component],
        frames: &[Frame],
    ) -> StacktraceState {
        let mut stacktrace_state = StacktraceState::default();

        // Apply direct frame actions and update the stack state alongside
        for rule in &self.updater_rules {
            for idx in 0..frames.len() {
                if rule.matches_frame(frames, idx) {
                    rule.update_frame_components_contributions(components, frames, idx);
                    rule.modify_stacktrace_state(&mut stacktrace_state);
                }
            }
        }
        // Use the stack state to update frame contributions again to trim
        // down to max-frames.  min-frames is handled on the other hand for
        // the entire stacktrace later.
        let max_frames = stacktrace_state.max_frames.value;

        if max_frames > 0 {
            let mut ignored = 0;

            for component in components.iter_mut().rev() {
                if !component.contributes {
                    continue;
                }

                ignored += 1;

                if ignored <= max_frames {
                    continue;
                }

                let hint = format!(
                    "ignored because only {} {} considered",
                    max_frames,
                    if max_frames != 1 {
                        "frames are"
                    } else {
                        "frame is"
                    },
                );

                let hint = stacktrace_state
                    .max_frames
                    .setter
                    .as_ref()
                    .map(|r| format!("{hint} by stack trace rule ({r})"))
                    .unwrap_or(hint);

                component.contributes = false;
                component.hint = Some(hint);
            }
        }

        stacktrace_state
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

            if rule.has_updater_action() {
                self.updater_rules.push(rule.clone());
            }

            self.all_rules.push(rule);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Component {
    pub contributes: bool,
    pub is_prefix_frame: bool,
    pub is_sentinel_frame: bool,
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct StacktraceVariable<T> {
    pub value: T,
    pub setter: Option<Rule>,
}

#[derive(Debug, Clone, Default)]
pub struct StacktraceState {
    pub max_frames: StacktraceVariable<usize>,
    pub min_frames: StacktraceVariable<usize>,
    pub invert_stacktrace: StacktraceVariable<bool>,
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
