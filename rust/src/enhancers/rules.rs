use std::fmt;
use std::sync::Arc;

use super::actions::Action;
use super::frame::Frame;
use super::matchers::{ExceptionMatcher, FrameMatcher};
use super::ExceptionData;

#[derive(Clone)]
pub struct Rule {
    pub frame_matchers: Vec<Arc<dyn FrameMatcher>>,
    pub exception_matchers: Vec<Arc<dyn ExceptionMatcher>>,
    pub actions: Vec<Action>,
}

impl fmt::Debug for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rule")
            .field("frame_matchers", &self.frame_matchers.len())
            .field("exception_matchers", &self.exception_matchers.len())
            .field("actions", &self.actions)
            .finish()
    }
}

impl Rule {
    pub fn matches_exception(&self, exception_data: &ExceptionData) -> bool {
        self.exception_matchers
            .iter()
            .all(|m| m.matches_exception(exception_data))
    }

    pub fn matches_frame(&self, frames: &[Frame], idx: usize) -> bool {
        self.frame_matchers
            .iter()
            .all(|m| m.matches_frame(frames, idx))
    }

    pub fn has_modifier_action(&self) -> bool {
        self.actions.iter().any(|a| a.is_modifier())
    }

    pub fn apply_modifications_to_frame(&self, frames: &mut [Frame], idx: usize) {
        for action in &self.actions {
            action.apply_modifications_to_frame(frames, idx)
        }
    }
}
