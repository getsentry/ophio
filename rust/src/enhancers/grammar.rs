//! Parse enhancement rules from the string representation.
//!
//! The parser is made using the `nom` parser combinator library.
//! The grammar was adapted to `nom` from:
//! <https://github.com/getsentry/sentry/blob/e5c5e56d176d96081ce4b25424e6ec7d3ba17cff/src/sentry/grouping/enhancer/__init__.py#L42-L79>

// TODO:
// - we should probably support better Error handling
// - quoted identifiers/arguments should properly support escapes, etc

use std::fmt;

use nom::branch::alt;
use nom::bytes::complete::{tag, take_while1};
use nom::character::complete::{anychar, char, space0};
use nom::combinator::{all_consuming, cut, map, map_res, opt, value};
use nom::error::{context, ErrorKind, VerboseError};
use nom::multi::{many0, many1};
use nom::sequence::{delimited, preceded, tuple};
use nom::{Finish, IResult, Parser};
use smol_str::SmolStr;

use super::actions::{Action, FlagAction, FlagActionType, Range, VarAction};
use super::matchers::{FrameOffset, Matcher};
use super::rules::Rule;
use super::Cache;

#[derive(Debug)]
enum ParseErrorKind {
    Char(char),
    Nom(ErrorKind),
    External(anyhow::Error),
    Context(&'static str),
}

#[derive(Debug)]
struct ParseError<'a>(Vec<(&'a str, ParseErrorKind)>);

impl<'a> nom::error::ParseError<&'a str> for ParseError<'a> {
    fn from_error_kind(input: &'a str, kind: ErrorKind) -> Self {
        Self(vec![(input, ParseErrorKind::Nom(kind))])
    }

    fn append(input: &'a str, kind: ErrorKind, mut other: Self) -> Self {
        other.0.push((input, ParseErrorKind::Nom(kind)));
        other
    }

    fn from_char(input: &'a str, c: char) -> Self {
        Self(vec![(input, ParseErrorKind::Char(c))])
    }
}

impl<'a> nom::error::ContextError<&'a str> for ParseError<'a> {
    fn add_context(input: &'a str, ctx: &'static str, mut other: Self) -> Self {
        other.0.push((input, ParseErrorKind::Context(ctx)));
        other
    }
}

impl<'a> nom::error::FromExternalError<&'a str, anyhow::Error> for ParseError<'a> {
    fn from_external_error(input: &'a str, _kind: ErrorKind, e: anyhow::Error) -> Self {
        Self(vec![(input, ParseErrorKind::External(e))])
    }
}

impl fmt::Display for ParseError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Parse error:")?;
        for (input, error) in &self.0 {
            match error {
                ParseErrorKind::Nom(e) => writeln!(f, "{e:?} at: {input}")?,
                ParseErrorKind::Char(c) => writeln!(f, "expected '{c}' at: {input}")?,
                ParseErrorKind::Context(s) => writeln!(f, "in section '{s}', at: {input}")?,
                ParseErrorKind::External(e) => writeln!(f, "{e}")?,
            }
        }

        Ok(())
    }
}

impl std::error::Error for ParseError<'_> {}

type ParseResult<'a, T> = IResult<&'a str, T, ParseError<'a>>;

fn ident(input: &str) -> ParseResult<&str> {
    take_while1(|c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))(input)
}

fn rule_bool(input: &str) -> ParseResult<bool> {
    alt((
        value(true, alt((tag("1"), tag("yes"), tag("true")))),
        value(false, alt((tag("0"), tag("no"), tag("false")))),
    ))(input)
}

fn rule_number(input: &str) -> ParseResult<usize> {
    map(take_while1(|c: char| c.is_ascii_digit()), |n: &str| {
        n.parse().unwrap()
    })(input)
}

fn frame_matcher(frame_offset: FrameOffset) -> impl Fn(&str) -> ParseResult<Matcher> {
    move |input| {
        let input = input.trim_start();

        let quoted_ident = delimited(
            char('"'),
            take_while1(|c: char| {
                c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':' | '-' | ' ')
            }),
            char('"'),
        );
        let matcher_type = context("matcher type", alt((ident, quoted_ident)));

        let unquoted = take_while1(|c: char| !c.is_ascii_whitespace());
        // TODO: escapes, etc
        let quoted = delimited(char('"'), take_while1(|c: char| c != '"'), char('"'));
        let argument = context("matcher argument", alt((quoted, unquoted)));

        let mut matcher = map_res(
            tuple((opt(char('!')), matcher_type, char(':'), argument)),
            |(negated, matcher_type, _, argument): (_, _, _, &str)| {
                // TODO: support even more escapes
                let unescaped = argument.replace("\\\\", "\\");
                Matcher::new(
                    negated.is_some(),
                    matcher_type,
                    &unescaped,
                    frame_offset,
                    &mut Cache::default(),
                )
            },
        );

        matcher(input)
    }
}

fn matchers(input: &str) -> ParseResult<Vec<Matcher>> {
    let input = input.trim_start();

    let caller_matcher = context(
        "caller matcher",
        tuple((
            space0,
            char('['),
            space0,
            cut(frame_matcher(FrameOffset::Caller)),
            space0,
            char(']'),
            space0,
            char('|'),
        )),
    );
    let callee_matcher = context(
        "callee matcher",
        tuple((
            space0,
            char('|'),
            space0,
            char('['),
            space0,
            cut(frame_matcher(FrameOffset::Callee)),
            space0,
            char(']'),
        )),
    );

    let mut matchers = tuple((
        opt(caller_matcher),
        context("matchers", many1(frame_matcher(FrameOffset::None))),
        opt(callee_matcher),
    ));

    let (input, (caller_matcher, mut matchers, callee_matcher)) = matchers(input)?;

    if let Some((_, _, _, m, _, _, _, _)) = caller_matcher {
        matchers.push(m);
    }

    if let Some((_, _, _, _, _, m, _, _)) = callee_matcher {
        matchers.push(m);
    }

    Ok((input, matchers))
}

fn actions(input: &str) -> ParseResult<Vec<Action>> {
    let max_frames = preceded(
        tuple((tag("max-frames"), space0, char('='), space0)),
        rule_number,
    )
    .map(VarAction::MaxFrames);

    let min_frames = preceded(
        tuple((tag("min-frames"), space0, char('='), space0)),
        rule_number,
    )
    .map(VarAction::MinFrames);

    let invert_stacktrace = preceded(
        tuple((tag("invert-stacktrace"), space0, char('='), space0)),
        rule_bool,
    )
    .map(VarAction::InvertStacktrace);

    let category = preceded(tuple((tag("category"), space0, char('='), space0)), ident)
        .map(|c| VarAction::Category(SmolStr::new(c)));

    let var_action = context(
        "var action",
        alt((max_frames, min_frames, invert_stacktrace, category)),
    );

    let flag_name = alt((
        value(FlagActionType::Group, tag("group")),
        value(FlagActionType::App, tag("app")),
        value(FlagActionType::Prefix, tag("prefix")),
        value(FlagActionType::Sentinel, tag("sentinel")),
    ));
    let range = opt(alt((
        value(Range::Up, char('^')),
        value(Range::Down, char('v')),
    )));
    let flag = alt((value(true, char('+')), value(false, char('-'))));
    let flag_action = context(
        "flag action",
        tuple((range, flag, flag_name)).map(|(range, flag, ty)| FlagAction { range, flag, ty }),
    );

    let action = context(
        "action",
        preceded(
            space0,
            alt((map(flag_action, Action::Flag), map(var_action, Action::Var))),
        ),
    );

    let (input, actions) = many1(action)(input)?;

    Ok((input, actions))
}

/// Parses a [`Rule`] from its string representation.
pub fn parse_rule(input: &str) -> anyhow::Result<Rule> {
    let comment = tuple((space0, char('#'), many0(anychar)));
    let (_input, (matchers, actions, _)) =
        all_consuming(tuple((matchers, actions, opt(comment))))(input)
            .finish()
            .map_err(|e| anyhow::Error::msg(dbg!(e).to_string()))?;

    Ok(Rule::new(matchers, actions))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::enhancers::Frame;

    use super::*;

    #[test]
    fn parse_objc_matcher() {
        let rule = parse_rule(r#"stack.function:-[* -app"#).unwrap_err();

        println!("{rule:?}");

        // let frames = &[Frame::from_test(
        //     &json!({"function": "-[UIApplication sendAction:to:from:forEvent:] "}),
        //     "native",
        // )];
        // assert!(!rule.matches_frame(frames, 0));
    }
}
