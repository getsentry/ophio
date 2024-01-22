use std::fmt;
use std::sync::Arc;

use super::actions::Action;
use super::frame::Frame;
use super::matchers::{ExceptionMatcher, FrameMatcher, Matcher};
use super::{Component, ExceptionData, StacktraceState};

/// An enhancement rule, comprising exception matchers, frame matchers, and actions.
#[derive(Debug, Clone)]
pub struct Rule(pub(crate) Arc<RuleInner>);

#[derive(Debug, Clone)]
pub struct RuleInner {
    pub frame_matchers: Vec<FrameMatcher>,
    pub exception_matchers: Vec<ExceptionMatcher>,
    pub actions: Vec<Action>,
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for m in &self.0.exception_matchers {
            if !first {
                write!(f, " ")?;
            }
            write!(f, "{m}")?;
            first = false;
        }

        for m in &self.0.frame_matchers {
            if !first {
                write!(f, " ")?;
            }
            write!(f, "{m}")?;
            first = false;
        }

        for a in &self.0.actions {
            if !first {
                write!(f, " ")?;
            }
            write!(f, "{a}")?;
            first = false;
        }

        Ok(())
    }
}

impl Rule {
    pub fn new(matchers: Vec<Matcher>, actions: Vec<Action>) -> Self {
        let (mut frame_matchers, mut exception_matchers) = (Vec::new(), Vec::new());

        for m in matchers {
            match m {
                Matcher::Frame(m) => frame_matchers.push(m),
                Matcher::Exception(m) => exception_matchers.push(m),
            }
        }

        Self(Arc::new(RuleInner {
            frame_matchers,
            exception_matchers,
            actions,
        }))
    }

    /// Checks whether an exception matches this rule.
    pub fn matches_exception(&self, exception_data: &ExceptionData) -> bool {
        self.0
            .exception_matchers
            .iter()
            .all(|m| m.matches_exception(exception_data))
    }

    /// Checks whether the frame at `frames[idx]` matches this rule.
    pub fn matches_frame(&self, frames: &[Frame], idx: usize) -> bool {
        self.0
            .frame_matchers
            .iter()
            .all(|m| m.matches_frame(frames, idx))
    }

    /// Returns true if this rule contains any actions that may modify a stacktrace.
    pub fn has_modifier_action(&self) -> bool {
        self.0.actions.iter().any(|a| a.is_modifier())
    }

    pub fn has_updater_action(&self) -> bool {
        self.0.actions.iter().any(|a| a.is_updater())
    }

    pub fn modify_stacktrace_state(&self, state: &mut StacktraceState) {
        for a in &self.0.actions {
            a.modify_stacktrace_state(state, self.clone());
        }
    }

    /// Applies all modifications from this rule's actions to matching frames.
    pub fn apply_modifications_to_frame(&self, frames: &mut [Frame], idx: usize) {
        for action in &self.0.actions {
            action.apply_modifications_to_frame(frames, idx)
        }
    }

    pub fn update_frame_components_contributions(
        &self,
        components: &mut [Component],
        frames: &[Frame],
        idx: usize,
    ) {
        for action in &self.0.actions {
            action.update_frame_components_contributions(components, frames, idx, self);
        }
    }
}
