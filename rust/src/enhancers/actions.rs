use std::fmt;

use smol_str::SmolStr;

use super::frame::Frame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Range {
    Up,
    Down,
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Range::Up => write!(f, "^"),
            Range::Down => write!(f, "v"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagActionType {
    App,
    Group,
    Prefix,
    Sentinel,
}

impl fmt::Display for FlagActionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FlagActionType::App => write!(f, "app"),
            FlagActionType::Group => write!(f, "group"),
            FlagActionType::Prefix => write!(f, "prefix"),
            FlagActionType::Sentinel => write!(f, "sentinel"),
        }
    }
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

impl fmt::Display for FlagAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(range) = self.range {
            write!(f, "{range}")?;
        }

        write!(f, "{}{}", self.flag, self.ty)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarName {
    MinFrames,
    MaxFrames,
    Category,
    InvertStacktrace,
}

impl fmt::Display for VarName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VarName::MinFrames => write!(f, "min-frames"),
            VarName::MaxFrames => write!(f, "max-frames"),
            VarName::Category => write!(f, "category"),
            VarName::InvertStacktrace => write!(f, "invert-stacktrace"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VarAction {
    MinFrames(usize),
    MaxFrames(usize),
    Category(SmolStr),
    InvertStacktrace(bool),
}

impl fmt::Display for VarAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VarAction::MinFrames(value) => write!(f, "min-frames={value}"),
            VarAction::MaxFrames(value) => write!(f, "max-frames={value}"),
            VarAction::Category(value) => write!(f, "category={value}"),
            VarAction::InvertStacktrace(value) => write!(f, "invert-stacktrace={value}"),
        }
    }
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

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Flag(a) => a.fmt(f),
            Action::Var(a) => a.fmt(f),
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
