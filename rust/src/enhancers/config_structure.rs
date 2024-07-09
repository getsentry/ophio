//! Definition of the compact msgpack format for enhancements, and methods for deserializing it.

use std::borrow::Cow;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use super::actions::{Action, FlagAction, FlagActionType, Range, VarAction};
use super::frame::FrameField;
use super::matchers::{
    ExceptionMatcher, ExceptionMatcherType, FrameMatcher, FrameMatcherInner, FrameOffset, Matcher,
};
use super::{RegexCache, Rule};

/// The different flag action types, in the order in which they're encoded
/// as bitfields.
const FLAG_ACTION_TYPES: &[FlagActionType] = &[
    FlagActionType::Group,
    FlagActionType::App,
    FlagActionType::Prefix,
    FlagActionType::Sentinel,
];

/// The different flag action values and ranges, in the order in which
/// they're encoded as bitfields.
const FLAG_ACTION_VALUES: &[(bool, Option<Range>)] = &[
    (true, None),
    (true, Some(Range::Up)),
    (true, Some(Range::Down)),
    (false, None),
    (false, Some(Range::Up)),
    (false, Some(Range::Down)),
];

/// The offset (in bits) at which the encoded value & range starts
/// in an encoded flag action.
const FLAG_ACTION_VALUE_OFFSET: usize = 8;

/// Bitmask for the flag action type.
///
/// Note that this is 4 bits wide, even though we only need
/// 2 bits to encode the 4 types.
const FLAG_ACTION_TYPE_MASK: usize = 0xF;

/// Compact representation of an [`Enhancements`](super::Enhancements) structure.
///
/// Can be deserialized from msgpack.
#[derive(Debug, Deserialize, Serialize)]
pub struct EncodedEnhancements<'a>(
    pub usize,
    pub Vec<SmolStr>,
    #[serde(borrow)] pub Vec<EncodedRule<'a>>,
);

/// Compact representation of a [`Rule`].
///
/// Can be deserialized from msgpack.
#[derive(Debug, Deserialize, Serialize)]
pub struct EncodedRule<'a>(
    #[serde(borrow)] pub Vec<EncodedMatcher<'a>>,
    #[serde(borrow)] pub Vec<EncodedAction<'a>>,
);

impl<'a> EncodedRule<'a> {
    pub fn into_rule(self, regex_cache: &mut RegexCache) -> anyhow::Result<Rule> {
        let matchers = self
            .0
            .into_iter()
            .map(|encoded| EncodedMatcher::into_matcher(encoded, regex_cache))
            .collect::<anyhow::Result<_>>()?;
        let actions = self
            .1
            .into_iter()
            .map(EncodedAction::into_action)
            .collect::<anyhow::Result<_>>()?;

        Ok(Rule::new(matchers, actions))
    }

    /// Converts a [`Rule`] into its compressed form.
    #[allow(unused)]
    pub fn from_rule(rule: &Rule) -> Self {
        let matchers = rule
            .0
            .exception_matchers
            .iter()
            .map(EncodedMatcher::from_exception_matcher)
            .chain(
                rule.0
                    .frame_matchers
                    .iter()
                    .map(EncodedMatcher::from_frame_matcher),
            )
            .collect();

        let actions = rule
            .0
            .actions
            .iter()
            .map(EncodedAction::from_action)
            .collect();

        Self(matchers, actions)
    }
}

/// Compact representation of a [`Matcher`].
///
/// Can be deserialized from msgpack.
#[derive(Debug, Deserialize, Serialize)]
pub struct EncodedMatcher<'a>(pub Cow<'a, str>);

impl<'a> EncodedMatcher<'a> {
    /// Converts the encoded matcher to a [`Matcher`].
    ///
    /// The `cache` is used to memoize the computation of regexes.
    pub fn into_matcher(self, regex_cache: &mut RegexCache) -> anyhow::Result<Matcher> {
        let mut def = self.0.as_ref();
        let mut frame_offset = FrameOffset::None;

        if def.starts_with("|[") && def.ends_with(']') {
            frame_offset = FrameOffset::Callee;
            def = &def[2..def.len() - 1];
        } else if def.starts_with('[') && def.ends_with("]|") {
            frame_offset = FrameOffset::Caller;
            def = &def[1..def.len() - 2];
        }

        let (def, negated) = if let Some(def) = def.strip_prefix('!') {
            (def, true)
        } else {
            (def, false)
        };

        let mut families = String::new();
        let (key, arg) = match def.split_at(1) {
            ("p", arg) => ("path", arg),
            ("f", arg) => ("function", arg),
            ("m", arg) => ("module", arg),
            ("F", arg) => {
                use std::fmt::Write;
                for f in arg.chars() {
                    match f {
                        'N' => write!(&mut families, ",native").unwrap(),
                        'J' => write!(&mut families, ",javascript").unwrap(),
                        'a' => write!(&mut families, ",all").unwrap(),
                        _ => {}
                    }
                }
                ("family", families.get(1..).unwrap_or_default())
            }
            ("P", arg) => ("package", arg),
            ("a", arg) => ("app", arg),
            ("t", arg) => ("type", arg),
            ("v", arg) => ("value", arg),
            ("M", arg) => ("mechanism", arg),
            ("c", arg) => ("category", arg),
            _ => {
                anyhow::bail!("unable to parse encoded Matcher: `{}`", self.0)
            }
        };

        Matcher::new(negated, key, arg, frame_offset, regex_cache)
    }

    /// Converts an [`ExceptionMatcher`] into its compressed form.
    #[allow(unused)]
    pub fn from_exception_matcher(matcher: &ExceptionMatcher) -> Self {
        let ty = match matcher.ty {
            ExceptionMatcherType::Type => 't',
            ExceptionMatcherType::Value => 'v',
            ExceptionMatcherType::Mechanism => 'M',
        };

        let mut result = String::new();
        if matcher.negated {
            result.push('!')
        }

        result.push(ty);
        result.push_str(matcher.raw_pattern.as_str());

        Self(Cow::Owned(result))
    }

    /// Converts a [`FrameMatcher`] into its compressed form.
    #[allow(unused)]
    pub fn from_frame_matcher(matcher: &FrameMatcher) -> Self {
        let ty = match matcher.inner {
            FrameMatcherInner::Field { field, .. } | FrameMatcherInner::Noop { field } => {
                match field {
                    FrameField::Category => 'c',
                    FrameField::Function => 'f',
                    FrameField::Module => 'm',
                    FrameField::Package => 'P',
                    FrameField::Path => 'p',
                    FrameField::App => 'a',
                }
            }
            FrameMatcherInner::Family { .. } => 'F',
            FrameMatcherInner::InApp { .. } => 'a',
        };

        let mut result = String::new();
        match matcher.frame_offset {
            FrameOffset::Caller => result.push('['),
            FrameOffset::Callee => result.push_str("|["),
            FrameOffset::None => {}
        }

        if matcher.negated {
            result.push('!')
        }

        result.push(ty);
        result.push_str(matcher.raw_pattern.as_str());

        match matcher.frame_offset {
            FrameOffset::Caller => result.push_str("]|"),
            FrameOffset::Callee => result.push(']'),
            FrameOffset::None => {}
        }

        Self(Cow::Owned(result))
    }
}

/// The RHS of a [`VarAction`].
///
/// This wraps a `bool`, `usize`, or string according to the variable on the action's LHS.
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum VarActionValue {
    Int(usize),
    Bool(bool),
    Str(SmolStr),
}

/// Compact representation of an [`Action`].
///
/// Can be deserialized from msgpack.
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum EncodedAction<'a> {
    /// A flag action.
    ///
    /// # Encoding
    ///  The wrapped number encodes a flag action as follows:
    ///
    ///  The bits `b₁, b₀` encode which flag the action sets:
    ///
    ///| b₁b₀ |     flag   |
    ///| ---- | ---------- |
    ///|  00  |   `group`  |
    ///|  01  |    `app`   |
    ///|  10  |  `prefix`  |
    ///|  11  | `sentinel` |
    ///
    /// The bits `b10, b9, b8` encode the flag value and the range:
    ///
    ///| b₁₀b₉b₈ |   flag  |  range |
    ///| ------- | ------  | ------ |
    ///|   000   |  `true` | `none` |
    ///|   001   |  `true` |  `up`  |
    ///|   010   |  `true` | `down` |
    ///|   011   | `false` | `None` |
    ///|   100   | `false` |  `up`  |
    ///|   101   | `false` | `down` |
    ///
    /// All other bits are unused.
    FlagAction(usize),

    /// A [`VarAction`], comprising the name of the variable
    /// being set and the value it is set to.
    #[serde(borrow)]
    VarAction((&'a str, VarActionValue)),
}

impl<'a> EncodedAction<'a> {
    /// Converts the encoded action to an [`Action`].
    pub fn into_action(self) -> anyhow::Result<Action> {
        use VarActionValue::*;
        Ok(match self {
            EncodedAction::FlagAction(flag) => {
                let ty = FLAG_ACTION_TYPES
                    .get(flag & FLAG_ACTION_TYPE_MASK)
                    .copied()
                    .with_context(|| format!("Failed to convert encoded FlagAction: `{flag}`"))?;
                let (flag, range) = FLAG_ACTION_VALUES
                    .get(flag >> FLAG_ACTION_VALUE_OFFSET)
                    .copied()
                    .with_context(|| format!("Failed to convert encoded FlagAction: `{flag}`"))?;
                Action::Flag(FlagAction { flag, ty, range })
            }
            EncodedAction::VarAction(("min-frames", Int(value))) => {
                Action::Var(VarAction::MinFrames(value))
            }
            EncodedAction::VarAction(("max-frames", Int(value))) => {
                Action::Var(VarAction::MaxFrames(value))
            }
            EncodedAction::VarAction(("invert-stacktrace", Bool(value))) => {
                Action::Var(VarAction::InvertStacktrace(value))
            }
            EncodedAction::VarAction(("category", Str(value))) => {
                Action::Var(VarAction::Category(value.clone()))
            }
            _ => anyhow::bail!("Failed to convert encoded Action: `{:?}`", self),
        })
    }
}

impl EncodedAction<'static> {
    /// Converts an [`Action`] into its compressed form.
    pub fn from_action(action: &Action) -> Self {
        match action {
            Action::Flag(action) => {
                let ty = match action.ty {
                    FlagActionType::Group => 0b00,
                    FlagActionType::App => 0b01,
                    FlagActionType::Prefix => 0b10,
                    FlagActionType::Sentinel => 0b11,
                };

                let flag_range = match (action.flag, action.range) {
                    (true, None) => 0b000,
                    (true, Some(Range::Up)) => 0b001,
                    (true, Some(Range::Down)) => 0b010,
                    (false, None) => 0b011,
                    (false, Some(Range::Up)) => 0b100,
                    (false, Some(Range::Down)) => 0b101,
                };

                Self::FlagAction(flag_range << FLAG_ACTION_VALUE_OFFSET | ty)
            }
            Action::Var(action) => match action {
                VarAction::MinFrames(val) => {
                    Self::VarAction(("min-frames", VarActionValue::Int(*val)))
                }
                VarAction::MaxFrames(val) => {
                    Self::VarAction(("max-frames", VarActionValue::Int(*val)))
                }
                VarAction::Category(val) => {
                    Self::VarAction(("category", VarActionValue::Str(val.clone())))
                }
                VarAction::InvertStacktrace(val) => {
                    Self::VarAction(("invert-stacktrace", VarActionValue::Bool(*val)))
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::enhancers::grammar::parse_rule;

    use super::EncodedRule;

    #[test]
    fn test_error_value() {
        let input = r#"error.value:"*something*" max-frames=12"#;
        let rule = parse_rule(input, &mut Default::default()).unwrap();

        let serialized = rmp_serde::to_vec(&EncodedRule::from_rule(&rule)).unwrap();

        let deserialized: EncodedRule = rmp_serde::from_slice(&serialized).unwrap();

        assert_eq!(
            deserialized.into_rule(&mut Default::default()).unwrap(),
            rule
        );
    }

    #[test]
    fn test_in_app() {
        for pat in ["yes", "true", "1", "no", "false", "0"] {
            let input = format!("app:{pat} max-frames=12");
            let rule = parse_rule(&input, &mut Default::default()).unwrap();

            let serialized = rmp_serde::to_vec(&EncodedRule::from_rule(&rule)).unwrap();

            let deserialized: EncodedRule = rmp_serde::from_slice(&serialized).unwrap();
            let decoded = deserialized.into_rule(&mut Default::default()).unwrap();

            assert_eq!(decoded, rule);
        }
    }
}
