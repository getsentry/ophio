//! Enhancement rules, which can match frames and exceptions according to the matchers they contain
//! and perform actions on matching frames.

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
/// The inner value of a [`Rule`], containing its matchers and actions.
pub struct RuleInner {
    /// The rule's frame matchers.
    pub frame_matchers: Vec<FrameMatcher>,
    /// The rule's exception matchers.
    pub exception_matchers: Vec<ExceptionMatcher>,
    /// The rule's actions.
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
    /// Creates a `Rule` from a vector of [`Matchers`](Matcher) and a vector of [`Actions`](Action).
    ///
    /// The matchers are internally sorted into exception and frame matchers.
    pub(crate) fn new(matchers: Vec<Matcher>, actions: Vec<Action>) -> Self {
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

    /// Checks whether an exception matches this rule, i.e., if it matches all exception matchers.
    ///
    /// This defaults to `true` if no exception matcher exists.
    pub fn matches_exception(&self, exception_data: &ExceptionData) -> bool {
        self.0
            .exception_matchers
            .iter()
            .all(|m| m.matches_exception(exception_data))
    }

    /// Checks whether the frame at `frames[idx]` matches this rule, i.e., if it matches all frame matchers.
    ///
    /// This defaults to `true` if no frame matcher exists.
    pub fn matches_frame(&self, frames: &[Frame], idx: usize) -> bool {
        self.0
            .frame_matchers
            .iter()
            .all(|m| m.matches_frame(frames, idx))
    }

    /// Returns true if this rule contains any actions that may modify the contents of frames.
    pub fn has_modifier_action(&self) -> bool {
        self.0.actions.iter().any(|a| a.is_modifier())
    }

    /// Returns true if this rule contains any actions that may update grouping contribution information.
    pub fn has_updater_action(&self) -> bool {
        self.0.actions.iter().any(|a| a.is_updater())
    }

    /// Modifies a [`StacktraceState`] according to the actions contained in this rule.
    pub fn modify_stacktrace_state(&self, state: &mut StacktraceState) {
        for a in &self.0.actions {
            a.modify_stacktrace_state(state, self.clone());
        }
    }

    /// Applies all modifications from this rule's actions to `frames` at the index `idx`.
    pub fn apply_modifications_to_frame(&self, frames: &mut [Frame], idx: usize) {
        for action in &self.0.actions {
            action.apply_modifications_to_frame(frames, idx, self)
        }
    }

    /// Updates grouping component contribution information.
    pub fn update_frame_components_contributions(&self, components: &mut [Component], idx: usize) {
        for action in &self.0.actions {
            action.update_frame_components_contributions(components, idx, self);
        }
    }
}
