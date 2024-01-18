use std::fmt;
use std::sync::Arc;

use super::actions::Action;
use super::frame::Frame;
use super::matchers::{ExceptionMatcher, FrameMatcher};
use super::ExceptionData;

#[derive(Clone)]
pub struct Rule(pub(crate) Arc<RuleInner>);

pub struct RuleInner {
    pub frame_matchers: Vec<FrameMatcher>,
    pub exception_matchers: Vec<ExceptionMatcher>,
    pub actions: Vec<Action>,
}

impl fmt::Debug for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rule")
            .field("frame_matchers", &self.0.frame_matchers.len())
            .field("exception_matchers", &self.0.exception_matchers.len())
            .field("actions", &self.0.actions)
            .finish()
    }
}

impl Rule {
    pub fn matches_exception(&self, exception_data: &ExceptionData) -> bool {
        self.0
            .exception_matchers
            .iter()
            .all(|m| m.matches_exception(exception_data))
    }

    pub fn matches_frame(&self, frames: &[Frame], idx: usize) -> bool {
        self.0
            .frame_matchers
            .iter()
            .all(|m| m.matches_frame(frames, idx))
    }

    pub fn has_modifier_action(&self) -> bool {
        self.0.actions.iter().any(|a| a.is_modifier())
    }

    pub fn apply_modifications_to_frame(&self, frames: &mut [Frame], idx: usize) {
        for action in &self.0.actions {
            action.apply_modifications_to_frame(frames, idx)
        }
    }
}
