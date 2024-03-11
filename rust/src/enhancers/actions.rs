//! Actions, which can modify the contents of stack frames or update grouping component contribution
//! information.
//!
//! See <https://docs.sentry.io/product/data-management-settings/event-grouping/stack-trace-rules/#actions> for an explanation of
//! the different types of actions.

use std::fmt;

use smol_str::SmolStr;

use super::{frame::Frame, Component, Rule, StacktraceState};

/// The range of an action.
///
/// This determines if the action applies to the frames/components before or after the current one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Range {
    /// The frames/components after the current one.
    Up,
    /// The frames/components before the current one.
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

/// The name of the flag a [`FlagAction`] sets.
///
/// The `app` flag is the only one of these that exists on stack frames,
/// the others belong to grouping components.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagActionType {
    /// The `app` flag.
    App,
    /// The `group` flag.
    Group,
    /// The `prefix` flag.
    Prefix,
    /// The `sentinel` flag.
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

/// A flag action.
///
/// It comprises three pieces of information:
/// * which flag it sets;
/// * whether it sets it to `true` or `false`;
/// * whether it sets the flag on the current frame/compoent, all previous ones,
///   or all following ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlagAction {
    /// The value the flag is set to.
    pub flag: bool,
    /// Which flag is set by this action.
    pub ty: FlagActionType,
    /// Which frames/components this action applies to.
    ///
    /// `None` means the current one, otherwise see the documentation of `Range`.
    pub range: Option<Range>,
}

impl FlagAction {
    /// Returns a mutable iterator over a subslice of `items`, depending on `self.range`.
    ///
    /// * `self.range` == `None`: returns just `items[idx]`, if it exists.
    /// * `self.range` == `Some(Up)`: returns `items[idx+1..]`.
    /// * `self.range` == `Some(Down)`: returns `items[..idx]`.
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

    /// Applies this action's modification to `frames` at the index `idx`.
    pub fn apply_modifications_to_frame(&self, frames: &mut [Frame], idx: usize, rule: &Rule) {
        if self.ty == FlagActionType::App {
            for frame in self.slice_to_range_mut(frames, idx) {
                frame.in_app = Some(self.flag);
                frame.in_app_last_changed = Some(rule.clone());
            }
        }
    }

    /// Updates grouping component contribution information according to this action.
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
                    if component.contributes != Some(self.flag) {
                        component.contributes = Some(self.flag);
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

/// A variable action.
///
/// These actions set variables to values. The type of the
/// value depends on the variable. The variable set by a var action
/// may be a property of a frame (`category`) or of the whole
/// [`StacktraceState`] (the rest).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VarAction {
    /// The `min-frames` variable on a [`StacktraceState`].
    ///
    /// The value must be a number.
    MinFrames(usize),
    /// The `max-frames` variable on a [`StacktraceState`].
    ///
    /// The value must be a number.
    MaxFrames(usize),
    /// The `category` variable on a [`Frame`].
    ///
    /// The value must be a string.
    Category(SmolStr),
    /// The `invert-stacktrace` variable on a [`StacktraceState`].
    ///
    /// The value must be a boolean.
    InvertStacktrace(bool),
}

impl VarAction {
    /// Applies this action's modification to `frames` at the index `idx`.
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

/// An action.
///
/// Every action is either a [`VarAction`] or a [`FlagAction`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Flag(FlagAction),
    Var(VarAction),
}

impl Action {
    /// Returns true if this action modifies a stacktrace.
    ///
    /// This is the case for the `app` flag action and the `category` var action.
    pub fn is_modifier(&self) -> bool {
        matches!(
            self,
            Action::Flag(FlagAction {
                ty: FlagActionType::App,
                ..
            },) | Action::Var(VarAction::Category(_))
        )
    }

    /// Returns true if this action updates stacktrace or component metadata.
    ///
    /// This is true for all actions except the `category` var action.
    pub fn is_updater(&self) -> bool {
        !matches!(self, Action::Var(VarAction::Category(_)))
    }

    /// Applies this action's modification to `frames` at the index `idx`.
    pub fn apply_modifications_to_frame(&self, frames: &mut [Frame], idx: usize, rule: &Rule) {
        match self {
            Action::Flag(action) => action.apply_modifications_to_frame(frames, idx, rule),
            Action::Var(action) => action.apply_modifications_to_frame(frames, idx),
        }
    }

    /// Updates grouping component contribution information according to this action.
    ///
    /// This is a no-op for var actions.
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

    /// Modifies stacktrace state metadata according to this action.
    ///
    /// This is only relevant for var actions that update the `min-frames`, `max-frames`
    /// or `invert-stacktrace` variables, otherwise it is a no-op.
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
