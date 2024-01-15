use std::sync::Arc;

use super::matchers::{ExceptionData, ExceptionMatcher, Frame, FrameMatcher};

#[derive(Clone)]
struct Rule {
    frame_matchers: Vec<Arc<dyn FrameMatcher>>,
    exception_matchers: Vec<Arc<dyn ExceptionMatcher>>,
    // TODO: Add actions
}

impl Rule {
    // TODO: Return actions
    fn get_actions(&self, frames: &[Frame], exception_data: &ExceptionData) -> Vec<usize> {
        if self
            .exception_matchers
            .iter()
            .any(|m| !m.matches_exception(exception_data))
        {
            return Vec::new();
        }

        let mut res = Vec::new();
        for idx in 0..frames.len() {
            if self
                .frame_matchers
                .iter()
                .all(|m| m.matches_frame(frames, idx))
            {
                res.push(idx);
            }
        }

        res
    }
}
