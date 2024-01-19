use std::fmt;
use std::sync::Arc;

use super::actions::Action;
use super::frame::Frame;
use super::matchers::{ExceptionMatcher, FrameMatcher};
use super::ExceptionData;

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
