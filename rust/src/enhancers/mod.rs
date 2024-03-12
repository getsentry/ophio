//! Functionality for creating grouping enhancements and applying them to stacktraces.
//!
//! This module's core type is [`Enhancements`]. It contains a list of [`Rules`](Rule) which can
//! modify certain stack frames and change grouping metadata.
//!
//! `Enhancements` can be parsed from a human-readable string representation with [`parse`](Enhancements::parse)
//! or from a compact msgpack representation with [`from_config_structure`](Enhancements::from_config_structure).
//!
//! They are applied to stacktraces with [`apply_modifications_to_frames`](Enhancements::apply_modifications_to_frames).

use std::fmt::Write;

use smol_str::SmolStr;

mod actions;
mod cache;
mod config_structure;
mod families;
mod frame;
mod grammar;
mod matchers;
mod rules;

pub use cache::*;
use config_structure::{EncodedAction, EncodedEnhancements, EncodedMatcher};
pub use families::Families;
pub use frame::{Frame, StringField};
pub use rules::Rule;

/// Exception data to match against rules.
#[derive(Debug, Clone, Default)]
pub struct ExceptionData {
    /// The exception's type, i.e. name.
    pub ty: Option<SmolStr>,
    /// The exception's value, i.e. human-readable description.
    pub value: Option<SmolStr>,
    /// The exception's mechanism.
    pub mechanism: Option<SmolStr>,
}

/// The result of the `assemble_stacktrace_component` fn.
pub struct AssembleResult {
    pub contributes: bool,
    pub hint: Option<String>,
    pub invert_stacktrace: bool,
}

/// A collection of [Rules](Rule) that modify the stacktrace and update grouping information.
#[derive(Debug, Default)]
pub struct Enhancements {
    /// The list of all rules in this collection.
    pub(crate) all_rules: Vec<Rule>,
    /// The list of "modifier rules" in this collection.
    ///
    /// Modifier rules are those rules that may modify a stacktrace.
    modifier_rules: Vec<Rule>,
    /// The list of "updater rules" in this collection.
    ///
    /// Updater rules are those rules that may update grouping metadata.
    updater_rules: Vec<Rule>,
}

impl Enhancements {
    /// Creates a new `Enhancements` from a list of `Rules`.
    pub fn new(all_rules: Vec<Rule>) -> Self {
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

        Enhancements {
            all_rules,
            modifier_rules,
            updater_rules,
        }
    }

    /// Parses an `Enhancements` structure from a string (in the form of a list of rules).
    pub fn parse(input: &str, cache: &mut Cache) -> anyhow::Result<Self> {
        let mut all_rules = vec![];

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let rule = cache.get_or_try_insert_rule(line)?;
            all_rules.push(rule);
        }

        Ok(Enhancements::new(all_rules))
    }

    /// Parses an `Enhancements` structure from the msgpack representation.
    pub fn from_config_structure(input: &[u8], cache: &mut Cache) -> anyhow::Result<Self> {
        let EncodedEnhancements(version, _bases, rules) = rmp_serde::from_slice(input)?;

        anyhow::ensure!(
            version == 2,
            "Rust Enhancements only supports config_structure version `2`"
        );

        let all_rules: Vec<_> = rules
            .into_iter()
            .map(|r| {
                let matchers =
                    r.0.into_iter()
                        .map(|encoded| EncodedMatcher::into_matcher(encoded, &mut cache.regex))
                        .collect::<anyhow::Result<_>>()?;
                let actions =
                    r.1.into_iter()
                        .map(EncodedAction::into_action)
                        .collect::<anyhow::Result<_>>()?;

                Ok(Rule::new(matchers, actions))
            })
            .collect::<anyhow::Result<_>>()?;

        Ok(Enhancements::new(all_rules))
    }

    /// Matches `frames` and `exception_data` against all rules in this collection
    /// and applies the corresponding modifications if a frame matches a rule.
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

    /// Assembles a `stacktrace` grouping component out of the given
    /// `frame` [`Component`]s and [`Frame`]s.
    ///
    /// It also updates the [`Component`]s `contributes`, `hint` and other attributes.
    pub fn assemble_stacktrace_component(
        &self,
        components: &mut [Component],
        frames: &[Frame],
        exception_data: &ExceptionData,
    ) -> AssembleResult {
        let mut stacktrace_state = StacktraceState::default();

        // First, update the `in_app` hints. We have kept track of which rule last set the
        // `in_app` field of each frame, so we don't need to iterate over the rules again for this.
        for (component, frame) in components.iter_mut().zip(frames.iter()) {
            if let Some(rule) = frame.in_app_last_changed.as_ref() {
                // in_app is definitely set, otherwise `in_app_last_changed` would be `None`
                let state = if frame.in_app.unwrap() {
                    "in-app"
                } else {
                    "out of app"
                };
                component.hint = Some(format!("marked {state} by stack trace rule ({rule})"));
            }
        }

        // Apply direct frame actions and update the stack state alongside
        for rule in &self.updater_rules {
            if !rule.matches_exception(exception_data) {
                continue;
            }

            for idx in 0..frames.len() {
                if rule.matches_frame(frames, idx) {
                    rule.update_frame_components_contributions(components, idx);
                    rule.modify_stacktrace_state(&mut stacktrace_state);
                }
            }
        }

        // Use the stack state to update frame contributions again to trim
        // down to `max-frames`.
        update_components_for_max_frames(components, stacktrace_state.max_frames);

        // `min-frames` is handled on the other hand for
        // the entire stacktrace.
        let (contributes, hint) =
            update_components_for_min_frames(components, stacktrace_state.min_frames);

        AssembleResult {
            contributes,
            hint,
            invert_stacktrace: stacktrace_state.invert_stacktrace.value,
        }
    }

    /// Returns an iterator over all rules in this collection.
    pub fn rules(&self) -> impl Iterator<Item = &Rule> {
        self.all_rules.iter()
    }

    /// Adds all rules contained in `other` to `self`.
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
    pub contributes: Option<bool>,
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

fn update_components_for_max_frames(
    components: &mut [Component],
    max_frames: StacktraceVariable<usize>,
) {
    let StacktraceVariable {
        value: max_frames,
        setter,
    } = max_frames;

    if max_frames == 0 {
        return;
    }

    let mut ignored = 0;

    for component in components.iter_mut().rev() {
        if !component.contributes.unwrap_or_default() {
            continue;
        }

        ignored += 1;

        if ignored <= max_frames {
            continue;
        }

        let mut hint = format!(
            "ignored because only {} {} considered",
            max_frames,
            if max_frames != 1 {
                "frames are"
            } else {
                "frame is"
            },
        );

        if let Some(rule) = &setter {
            write!(&mut hint, " by stack trace rule ({rule})").unwrap();
        }

        component.contributes = Some(false);
        component.hint = Some(hint);
    }
}

fn update_components_for_min_frames(
    components: &[Component],
    min_frames: StacktraceVariable<usize>,
) -> (bool, Option<String>) {
    let total_contributes: usize = components
        .iter()
        .map(|c| c.contributes.unwrap_or_default() as usize)
        .sum();

    let mut hint = None;
    let mut contributes = total_contributes > 0;

    let StacktraceVariable {
        value: min_frames,
        setter,
    } = min_frames;

    if min_frames == 0 {
        return (contributes, hint);
    }

    if total_contributes > 0 && total_contributes < min_frames {
        let mut hint_str = format!("discarded because stack trace only contains {total_contributes} frame{} which is under the configured threshold", if total_contributes == 1 { "" } else {"s"});

        if let Some(rule) = setter {
            write!(&mut hint_str, " by stack trace rule ({rule})").unwrap();
        }

        contributes = false;
        hint = Some(hint_str);
    }

    (contributes, hint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_enhancers() {
        let enhancers =
            std::fs::read_to_string("../tests/fixtures/newstyle@2023-01-11.txt").unwrap();
        let enhancements = Enhancements::parse(&enhancers, &mut Cache::default()).unwrap();
        dbg!(enhancements.all_rules.len());
        dbg!(enhancements.modifier_rules.len());
        dbg!(enhancements.updater_rules.len());
    }

    #[test]
    fn parses_encoded_default_enhancers() {
        let enhancers = std::fs::read("../tests/fixtures/newstyle@2023-01-11.bin").unwrap();
        let _enhancements =
            Enhancements::from_config_structure(&enhancers, &mut Cache::default()).unwrap();
    }
}
