use anyhow::Context;
use serde::Deserialize;
use smol_str::SmolStr;

use super::actions::{Action, FlagAction, FlagActionType, Range, VarAction};
use super::matchers::{FrameOffset, Matcher};

#[derive(Debug, Deserialize)]
pub struct EnhancementsStructure<'a>(
    pub usize,
    pub Vec<SmolStr>,
    #[serde(borrow)] pub Vec<RuleStructure<'a>>,
);

#[derive(Debug, Deserialize)]
pub struct RuleStructure<'a>(
    #[serde(borrow)] pub Vec<MatchStructure<'a>>,
    #[serde(borrow)] pub Vec<ActionStructure<'a>>,
);

#[derive(Debug, Deserialize)]
pub struct MatchStructure<'a>(pub &'a str);

impl<'a> MatchStructure<'a> {
    pub fn into_matcher(self) -> anyhow::Result<Matcher> {
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

        Matcher::new(negated, key, arg, frame_offset)
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum VarActionValue {
    Int(usize),
    Bool(bool),
    Str(SmolStr),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ActionStructure<'a> {
    FlagAction(usize),
    #[serde(borrow)]
    VarAction((&'a str, VarActionValue)),
}

impl<'a> ActionStructure<'a> {
    pub fn into_action(self) -> anyhow::Result<Action> {
        use VarActionValue::*;
        Ok(match self {
            ActionStructure::FlagAction(flag) => {
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
            ActionStructure::VarAction(("min-frames", Int(value))) => {
                Action::Var(VarAction::MinFrames(value))
            }
            ActionStructure::VarAction(("max-frames", Int(value))) => {
                Action::Var(VarAction::MaxFrames(value))
            }
            ActionStructure::VarAction(("invert-stacktrace", Bool(value))) => {
                Action::Var(VarAction::InvertStacktrace(value))
            }
            ActionStructure::VarAction(("category", Str(value))) => {
                Action::Var(VarAction::Category(value.clone()))
            }
            _ => anyhow::bail!("Failed to convert encoded Action: `{:?}`", self),
        })
    }
}
