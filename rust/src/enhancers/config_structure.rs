use anyhow::Context;
use serde::Deserialize;
use smol_str::SmolStr;

use super::actions::{Action, FlagAction, FlagActionType, Range, VarAction};
use super::matchers::{FrameOffset, Matcher};

#[derive(Debug, Deserialize)]
struct RuleStructure<'a>(
    #[serde(borrow)] Vec<MatchStructure<'a>>,
    #[serde(borrow)] Vec<ActionStructure<'a>>,
);

impl<'a> RuleStructure<'a> {
    fn from_msgpack_slice(slice: &'a [u8]) -> anyhow::Result<Self> {
        Ok(rmp_serde::from_slice(slice)?)
    }
}

#[derive(Debug, Deserialize)]
struct MatchStructure<'a>(&'a str);

impl<'a> MatchStructure<'a> {
    fn from_msgpack_slice(slice: &'a [u8]) -> anyhow::Result<Self> {
        Ok(rmp_serde::from_slice(slice)?)
    }

    fn into_matcher(self) -> anyhow::Result<Matcher> {
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
enum VarActionValue {
    Int(usize),
    Bool(bool),
    Str(SmolStr),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ActionStructure<'a> {
    FlagAction(usize),
    #[serde(borrow)]
    VarAction((&'a str, VarActionValue)),
    /*
    ACTIONS = ["group", "app", "prefix", "sentinel"]
    ACTION_BITSIZE = {
        # version -> bit-size
        1: 4,
        2: 8,
    }
    assert len(ACTIONS) < 1 << max(ACTION_BITSIZE.values())
    ACTION_FLAGS = {
        (True, None): 0,
        (True, "up"): 1,
        (True, "down"): 2,
        (False, None): 3,
        (False, "up"): 4,
        (False, "down"): 5,
    }
    REVERSE_ACTION_FLAGS = {v: k for k, v in ACTION_FLAGS.items()}

        @classmethod
        def _from_config_structure(cls, val, version: int):
            if isinstance(val, list):
                return VarAction(val[0], val[1])
            flag, range = REVERSE_ACTION_FLAGS[val >> ACTION_BITSIZE[version]]
            return FlagAction(ACTIONS[val & 0xF], flag, range)
        */
}

impl<'a> ActionStructure<'a> {
    fn from_msgpack_slice(slice: &'a [u8]) -> anyhow::Result<Self> {
        Ok(rmp_serde::from_slice(slice)?)
    }

    fn into_action(self) -> anyhow::Result<Action> {
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
