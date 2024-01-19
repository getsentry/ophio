use smol_str::SmolStr;

use super::frame::Frame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Range {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagActionType {
    App,
    Group,
    Prefix,
    Sentinel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlagAction {
    pub flag: bool,
    pub ty: FlagActionType,
    pub range: Option<Range>,
}

impl FlagAction {
    fn iter_frames<'f>(
        &self,
        frames: &'f mut [Frame],
        idx: usize,
    ) -> impl Iterator<Item = &'f mut Frame> {
        let slice = match self.range {
            Some(Range::Up) => frames.get_mut(idx + 1..),
            Some(Range::Down) => frames.get_mut(..idx),
            None => frames.get_mut(idx..idx + 1),
        };
        slice.unwrap_or_default().iter_mut()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarName {
    MinFrames,
    MaxFrames,
    Category,
    InvertStacktrace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VarAction {
    MinFrames(usize),
    MaxFrames(usize),
    Category(SmolStr),
    InvertStacktrace(bool),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Flag(FlagAction),
    Var(VarAction),
}

impl Action {
    pub fn is_modifier(&self) -> bool {
        matches!(
            self,
            Action::Flag(FlagAction {
                ty: FlagActionType::App,
                ..
            },) | Action::Var(VarAction::Category(_))
        )
    }

    pub fn apply_modifications_to_frame(&self, frames: &mut [Frame], idx: usize) {
        match self {
            Action::Flag(
                action @ FlagAction {
                    ty: FlagActionType::App,
                    ..
                },
            ) => {
                for frame in action.iter_frames(frames, idx) {
                    frame.in_app = action.flag;
                }
            }
            Action::Var(VarAction::Category(value)) => {
                if let Some(frame) = frames.get_mut(idx) {
                    frame.category = Some(value.clone())
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::enhancers::{Cache, Enhancements};

    use super::*;

    #[test]
    fn in_app_modification() {
        let enhancements = Enhancements::parse("app:no +app", &mut Cache::default()).unwrap();

        let mut frames = vec![
            Frame::from_test(&json!({"function": "foo"}), "native"),
            Frame::from_test(&json!({"function": "foo", "in_app": false}), "native"),
        ];

        enhancements.apply_modifications_to_frames(&mut frames, &Default::default());

        assert!(frames[0].in_app);
        assert!(frames[1].in_app);
    }
}
