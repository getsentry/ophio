use smol_str::SmolStr;

use super::frame::Frame;
use super::grammar::RawAction;

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
    flag: bool,
    ty: FlagActionType,
    range: Option<Range>,
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
pub struct VarAction {
    var: VarName,
    value: SmolStr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Flag(FlagAction),
    Var(VarAction),
}

impl Action {
    pub fn from_raw(raw: RawAction) -> Self {
        match raw {
            RawAction::Var(var_name, value) => {
                let var = match var_name.as_str() {
                    "max-frames" => VarName::MaxFrames,
                    "min-frames" => VarName::MinFrames,
                    "invert-stacktrace" => VarName::InvertStacktrace,
                    "category" => VarName::Category,
                    _ => unreachable!(),
                };

                Self::Var(VarAction { var, value })
            }
            RawAction::Flag(range, flag, ty) => {
                let range = range.map(|r| match r {
                    '^' => Range::Up,
                    _ => Range::Down,
                });

                let flag = flag == '+';

                let ty = match ty.as_str() {
                    "app" => FlagActionType::App,
                    "group" => FlagActionType::Group,
                    "prefix" => FlagActionType::Prefix,
                    "sentinel" => FlagActionType::Sentinel,
                    _ => unreachable!(),
                };

                Self::Flag(FlagAction { flag, ty, range })
            }
        }
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
            Action::Var(VarAction {
                var: VarName::Category,
                value,
            }) => {
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

    use crate::enhancers::Enhancements;

    use super::*;

    #[test]
    fn in_app_modification() {
        let enhancements = Enhancements::parse("app:no +app").unwrap();

        let mut frames = vec![
            Frame::from_test(json!({"function": "foo"}), "native"),
            Frame::from_test(json!({"function": "foo", "in_app": false}), "native"),
        ];

        enhancements.apply_modifications_to_frames(&mut frames, &Default::default());

        assert!(frames[0].in_app);
        assert!(frames[1].in_app);
    }
}
