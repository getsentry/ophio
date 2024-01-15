use smol_str::SmolStr;

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
    fn from_raw(raw: RawAction) -> Self {
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
}
