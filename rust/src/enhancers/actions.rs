use std::fmt;

use smol_str::SmolStr;

use super::{frame::Frame, Component, Rule, StacktraceState};

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
    pub fn apply_modifications_to_frame(&self, frames: &mut [Frame], idx: usize, rule: &Rule) {
        if self.ty == FlagActionType::App {
            for frame in self.slice_to_range_mut(frames, idx) {
                frame.in_app = Some(self.flag);
                frame.in_app_last_changed = Some(rule.clone());
            }
        }
    }

    fn update_frame_components_contributions(
        &self,
        components: &mut [Component],
        idx: usize,
        rule: &Rule,
    ) {
        let rule_hint = "stack trace rule";
        let components = self.slice_to_range_mut(components, idx);

        for component in components {
            match self.ty {
                FlagActionType::Group => {
                    if component.contributes != self.flag {
                        component.contributes = self.flag;
                        let state = if self.flag { "un-ignored" } else { "ignored" };
                        component.hint = Some(format!("{state} by {rule_hint} ({rule})"));
                    }
                }
                FlagActionType::App => {
                    //See Enhancements::update_frame_components_contributions
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
}

impl fmt::Display for FlagAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(range) = self.range {
            write!(f, "{range}")?;
        }

        write!(f, "{}{}", if self.flag { "+" } else { "-" }, self.ty)
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
                    frame.category = Some(value.clone());
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
    /// Returns true if this action modifies a stacktrace.
    pub fn is_modifier(&self) -> bool {
        matches!(
            self,
            Action::Flag(FlagAction {
                ty: FlagActionType::App,
                ..
            },) | Action::Var(VarAction::Category(_))
        )
    }

    pub fn is_updater(&self) -> bool {
        !matches!(self, Action::Var(VarAction::Category(_)))
    }

    /// Applies this action's modification to the given list of frames at the given index.
    pub fn apply_modifications_to_frame(&self, frames: &mut [Frame], idx: usize, rule: &Rule) {
        match self {
            Action::Flag(action) => action.apply_modifications_to_frame(frames, idx, rule),
            Action::Var(action) => action.apply_modifications_to_frame(frames, idx),
        }
    }

    pub fn update_frame_components_contributions(
        &self,
        components: &mut [Component],
        idx: usize,
        rule: &Rule,
    ) {
        if let Self::Flag(action) = self {
            action.update_frame_components_contributions(components, idx, rule);
        }
    }

    pub fn modify_stacktrace_state(&self, state: &mut StacktraceState, rule: Rule) {
        if let Self::Var(a) = self {
            match a {
                VarAction::Category(_) => (),
                VarAction::MinFrames(v) => {
                    state.min_frames.value = *v;
                    state.min_frames.setter = Some(rule);
                }
                VarAction::MaxFrames(v) => {
                    state.max_frames.value = *v;
                    state.max_frames.setter = Some(rule);
                }
                VarAction::InvertStacktrace(v) => {
                    state.invert_stacktrace.value = *v;
                    state.invert_stacktrace.setter = Some(rule);
                }
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

        assert_eq!(frames[0].in_app, Some(true));
        assert_eq!(frames[1].in_app, Some(true));
    }
}
