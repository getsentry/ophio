use std::{iter, sync::Arc};

use super::{
    actions::Action,
    grammar::{RawMatcher, RawRule},
    matchers::{
        get_matcher, ExceptionData, ExceptionMatcher, Frame, FrameMatcher, FrameOrExceptionMatcher,
    },
};

#[derive(Clone)]
struct Rule {
    frame_matchers: Vec<Arc<dyn FrameMatcher>>,
    exception_matchers: Vec<Arc<dyn ExceptionMatcher>>,
    actions: Vec<Action>,
}

impl Rule {
    fn from_raw(raw: RawRule) -> anyhow::Result<Self> {
        let mut frame_matchers = Vec::new();
        let mut exception_matchers = Vec::new();
        let mut actions = Vec::new();

        if let Some(RawMatcher {
            negation,
            ty,
            argument,
        }) = raw.matchers.caller_matcher
        {
            match get_matcher(negation, ty, argument)? {
                FrameOrExceptionMatcher::Frame(_) => todo!(),
                FrameOrExceptionMatcher::Exception(_) => todo!(),
            }
        }
    }
    fn get_actions(
        &self,
        frames: &[Frame],
        exception_data: &ExceptionData,
    ) -> Vec<(usize, Action)> {
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
                res.extend(iter::repeat(idx).zip(self.actions.iter().cloned()))
            }
        }

        res
    }
}
