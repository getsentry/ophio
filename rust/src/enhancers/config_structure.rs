//! Definition of the compact msgpack format for enhancements, and methods for deserializing it.

use anyhow::Context;
use serde::Deserialize;
use smol_str::SmolStr;

use super::actions::{Action, FlagAction, FlagActionType, Range, VarAction};
use super::matchers::{FrameOffset, Matcher};
use super::RegexCache;

/// Compact representation of an [`Enhancements`](super::Enhancements) structure.
///
/// Can be deserialized from msgpack.
#[derive(Debug, Deserialize)]
pub struct EncodedEnhancements<'a>(
    pub usize,
    pub Vec<SmolStr>,
    #[serde(borrow)] pub Vec<EncodedRule<'a>>,
);

/// Compact representation of a [`Rule`](super::rules::Rule).
///
/// Can be deserialized from msgpack.
#[derive(Debug, Deserialize)]
pub struct EncodedRule<'a>(
    #[serde(borrow)] pub Vec<EncodedMatcher<'a>>,
    #[serde(borrow)] pub Vec<EncodedAction<'a>>,
);

/// Compact representation of a [`Matcher`].
///
/// Can be deserialized from msgpack.
#[derive(Debug, Deserialize)]
pub struct EncodedMatcher<'a>(pub &'a str);

impl<'a> EncodedMatcher<'a> {
    /// Converts the encoded matcher to a [`Matcher`].
    ///
    /// The `cache` is used to memoize the computation of regexes.
    pub fn into_matcher(self, regex_cache: &mut RegexCache) -> anyhow::Result<Matcher> {
        let mut def = self.0;
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
}

/// The RHS of a [`VarAction`].
///
/// This wraps a `bool`, `usize`, or string according to the variable on the action's LHS.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum VarActionValue {
    Int(usize),
    Bool(bool),
    Str(SmolStr),
}

/// Compact representation of an [`Action`].
///
/// Can be deserialized from msgpack.
#[derive(Debug, Deserialize)]
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
                const ACTIONS: &[FlagActionType] = &[
                    FlagActionType::Group,
                    FlagActionType::App,
                    FlagActionType::Prefix,
                    FlagActionType::Sentinel,
                ];
                const FLAGS: &[(bool, Option<Range>)] = &[
                    (true, None),
                    (true, Some(Range::Up)),
                    (true, Some(Range::Down)),
                    (false, None),
                    (false, Some(Range::Up)),
                    (false, Some(Range::Down)),
                ];
                // NOTE: we only support version 2 encoding here
                const ACTION_BITSIZE: usize = 8;
                const ACTION_MASK: usize = 0xF;

                let ty = ACTIONS
                    .get(flag & ACTION_MASK)
                    .copied()
                    .with_context(|| format!("Failed to convert encoded FlagAction: `{flag}`"))?;
                let (flag, range) = FLAGS
                    .get(flag >> ACTION_BITSIZE)
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
