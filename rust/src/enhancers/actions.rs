use std::fmt;

use smol_str::SmolStr;

use super::{frame::Frame, Component, Rule};

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
    fn slice_to_range<'f, I>(&self, items: &'f [I], idx: usize) -> impl Iterator<Item = &'f I> {
        let slice = match self.range {
            Some(Range::Up) => items.get(idx + 1..),
            Some(Range::Down) => items.get(..idx),
            None => items.get(idx..idx + 1),
        };
        slice.unwrap_or_default().iter()
    }

    fn slice_to_range_mut<'f, I>(
        &self,
        items: &'f mut [I],
        idx: usize,
    ) -> impl Iterator<Item = &'f mut I> {
        let slice = match self.range {
            Some(Range::Up) => items.get_mut(idx + 1..),
            Some(Range::Down) => items.get_mut(..idx),
            None => items.get_mut(idx..idx + 1),
        };
        slice.unwrap_or_default().iter_mut()
    }

    /// Applies this action's modification to the given list of frames at the given index.
    pub fn apply_modifications_to_frame(&self, frames: &mut [Frame], idx: usize) {
        if self.ty == FlagActionType::App {
            for frame in self.slice_to_range_mut(frames, idx) {
                let orig_in_app = frame.in_app;
                if orig_in_app != self.flag {
                    frame.in_app = self.flag;
                    frame.orig_in_app = Some(orig_in_app);
                }
            }
        }
    }

    fn update_frame_components_contributions(
        &self,
        components: &mut [Component],
        frames: &[Frame],
        idx: usize,
        rule: &Rule,
    ) {
        let rule_hint = "stack trace rule";
        let components = self.slice_to_range_mut(components, idx);
        let frames = self.slice_to_range(frames, idx);

        for (component, frame) in components.zip(frames) {
            match self.ty {
                FlagActionType::Group => {
                    if component.contributes != self.flag {
                        component.contributes = self.flag;
                        let state = if self.flag { "un-ignored" } else { "ignored" };
                        component.hint = Some(format!("{state} by {rule_hint} ({rule})"));
                    }
                }
                FlagActionType::App => {
                    // The in app flag was set by `apply_modifications_to_frame`
                    // but we want to add a hint if there is none yet.
                    if self.in_app_changed(component, frame) {
                        let state = if self.flag { "in-app" } else { "out of app" };
                        component.hint = Some(format!("marked {state} by {rule_hint} ({rule})"));
                    }
                }
                FlagActionType::Prefix => {
                    component.is_prefix_frame = self.flag;
                    component.hint =
                        Some(format!("marked as prefix frame by {rule_hint} ({rule})"));
                }
                FlagActionType::Sentinel => {
                    component.is_sentinel_frame = self.flag;
                    component.hint =
                        Some(format!("marked as sentinel frame by {rule_hint} ({rule})"));
                }
            }
        }
    }

    fn in_app_changed(&self, component: &Component, frame: &Frame) -> bool {
        if let Some(orig_in_app) = frame.orig_in_app {
            orig_in_app != frame.in_app
        } else {
            self.flag == component.contributes
        }
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

impl VarAction {
    /// Applies this action's modification to the given list of frames at the given index.
    fn apply_modifications_to_frame(&self, frames: &mut [Frame], idx: usize) {
        {
            if let Self::Category(value) = self {
                if let Some(frame) = frames.get_mut(idx) {
                    frame.category = Some(value.clone())
                }
            }
        }
    }
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

    /// Applies this action's modification to the given list of frames at the given index.
    pub fn apply_modifications_to_frame(&self, frames: &mut [Frame], idx: usize) {
        match self {
            Action::Flag(action) => action.apply_modifications_to_frame(frames, idx),
            Action::Var(action) => action.apply_modifications_to_frame(frames, idx),
        }
    }

    pub fn update_frame_components_contributions(
        &self,
        components: &mut [Component],
        frames: &[Frame],
        idx: usize,
        rule: &Rule,
    ) {
        if let Self::Flag(action) = self {
            action.update_frame_components_contributions(components, frames, idx, rule);
        }
    }

    pub fn modify_stacktrace_state(&self, state: &mut StacktraceState, rule: Rule) {
        if let Self::Var(a) = self {
            match a {
                VarAction::MinFrames(v) => todo!(),
                VarAction::MaxFrames(v) => todo!(),
                VarAction::Category(v) => todo!(),
                VarAction::InvertStacktrace(v) => todo!(),
            }
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
