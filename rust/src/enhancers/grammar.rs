//! Parse enhancement rules from the string representation.
//!
//! We are using a hand-written recursive descent parser. The grammar is adapted from
//! <https://github.com/getsentry/sentry/blob/e5c5e56d176d96081ce4b25424e6ec7d3ba17cff/src/sentry/grouping/enhancer/__init__.py#L42-L79>

// TODO:
// - quoted identifiers/arguments should properly support escapes, etc

use std::borrow::Cow;

use anyhow::{anyhow, Context};

use super::actions::{Action, FlagAction, FlagActionType, Range, VarAction};
use super::matchers::{FrameOffset, Matcher};
use super::rules::Rule;
use super::RegexCache;

/// Possible prefixes of a matcher definition.
/// Matchers always start with one of these,
/// and actions never do. This means that if
/// the rest of the input starts with one these,
/// there is another matcher to parse, and if it doesn't,
/// there isn't.
const MATCHER_LOOKAHEAD: [&str; 11] = [
    "!",
    "a",
    "category:",
    "e",
    "f",
    "me",
    "mo",
    "p",
    "s",
    "t",
    "va",
];

/// Strips the prefix `pat` from `input` and returns the rest.
///
/// Returns an error if `input` doesn't start with `pat.`
fn expect<'a>(input: &'a str, pat: &str) -> anyhow::Result<&'a str> {
    input
        .strip_prefix(pat)
        .ok_or_else(|| anyhow!("at `{input}`: expected `{pat}`"))
}

/// Parses a string into a bool.
///
/// `"1"`, `"yes"`, and `"true"` parse to `true`,
/// `"0"`, `"no"`, and `"false"` parse to `false`,
/// and anything else is an error.
fn bool(input: &str) -> anyhow::Result<bool> {
    match input {
        "1" | "yes" | "true" => Ok(true),
        "0" | "no" | "false" => Ok(false),
        _ => anyhow::bail!("at `{input}`: invalid boolean value"),
    }
}

/// Parses an "identifier" and returns it together with the rest of the input.
///
/// An "identifier" is defined by the regex `[a-zA-Z0-9_.-]+`.
fn ident(input: &str) -> anyhow::Result<(&str, &str)> {
    let Some(end) =
        input.find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-')))
    else {
        return Ok((input, ""));
    };

    if end == 0 {
        anyhow::bail!("at `{input}`: invalid identifier");
    }

    Ok(input.split_at(end))
}

/// Parses an "argument", i.e., the right-hand side of a matcher definition, and returns it together
/// with the rest of the input.
///
/// An "argument" is either a sequence of non-whitespace ASCII characters or any sequence of
/// non-`"` characters enclosed in `""`.
///
/// Escaped characters in the argument are unescaped.
fn argument(input: &str) -> anyhow::Result<(Cow<str>, &str)> {
    let (result, rest) = if let Some(rest) = input.strip_prefix('"') {
        let end = rest
            .find('"')
            .ok_or_else(|| anyhow!("at `{input}`: unclosed `\"`"))?;
        let result = &rest[..end];
        let rest = &rest[end + 1..];
        (result, rest)
    } else {
        match input.find(|c: char| c.is_ascii_whitespace()) {
            None => (input, ""),
            Some(end) => input.split_at(end),
        }
    };

    // TODO: support even more escapes
    let unescaped = if result.contains("\\\\") {
        result.replace("\\\\", "\\").into()
    } else {
        result.into()
    };

    Ok((unescaped, rest))
}

/// Parses a [`VarAction`] and returns it together with the rest of the input.
fn var_action(input: &str) -> anyhow::Result<(VarAction, &str)> {
    let input = input.trim_start();

    let (lhs, after_lhs) =
        ident(input).with_context(|| format!("at `{input}`: expected variable name"))?;

    let after_lhs = after_lhs.trim_start();

    let after_eq = expect(after_lhs, "=")?.trim_start();

    let (rhs, rest) =
        ident(after_eq).with_context(|| format!("at `{after_eq}`: expected value for variable"))?;

    let a = match lhs {
        "max-frames" => {
            let n = rhs
                .parse()
                .with_context(|| format!("at `{rhs}`: failed to parse rhs of `max-frames`"))?;
            VarAction::MaxFrames(n)
        }

        "min-frames" => {
            let n = rhs
                .parse()
                .with_context(|| format!("at `{rhs}`: failed to parse rhs of `min-frames`"))?;
            VarAction::MinFrames(n)
        }

        "invert-stacktrace" => {
            let b = bool(rhs).with_context(|| {
                format!("at `{rhs}`: failed to parse rhs of `invert-stacktrace`")
            })?;
            VarAction::InvertStacktrace(b)
        }

        "category" => VarAction::Category(rhs.into()),

        _ => anyhow::bail!("at `{input}`: invalid variable name `{lhs}`"),
    };

    Ok((a, rest))
}

/// Parses a [`FlagAction`] and returns it together with the rest of the input.
fn flag_action(input: &str) -> anyhow::Result<(FlagAction, &str)> {
    let input = input.trim_start();

    let (range, after_range) = if let Some(rest) = input.strip_prefix('^') {
        (Some(Range::Up), rest)
    } else if let Some(rest) = input.strip_prefix('v') {
        (Some(Range::Down), rest)
    } else {
        (None, input)
    };

    let (flag, after_flag) = if let Some(rest) = after_range.strip_prefix('+') {
        (true, rest)
    } else if let Some(rest) = after_range.strip_prefix('-') {
        (false, rest)
    } else {
        anyhow::bail!("at `{input}`: expected flag value");
    };

    let (name, rest) =
        ident(after_flag).with_context(|| format!("at `{after_flag}`: expected flag name"))?;

    let ty = match name {
        "app" => FlagActionType::App,
        "group" => FlagActionType::Group,
        "prefix" => FlagActionType::Prefix,
        "sentinel" => FlagActionType::Sentinel,
        _ => anyhow::bail!("at `{after_flag}`: invalid flag name `{name}`"),
    };

    Ok((FlagAction { flag, ty, range }, rest))
}

/// Parses a sequence of [`Actions`](Action) and returns it.
///
/// The sequence must contain at least one action.
///
/// Since actions are the last part of a rule definition and can only
/// be followed by whitespace or a comment, there is no point in returning the
/// rest of the input.
fn actions(input: &str) -> anyhow::Result<Vec<Action>> {
    let mut input = input.trim_start();

    let mut result = Vec::new();

    // we're done with actions if there's either nothing or just a comment remaining.
    while !input.is_empty() && !input.starts_with('#') {
        // flag actions always start with one of these characters, and var actions never do.
        if input.starts_with(['v', '^', '+', '-']) {
            let (action, after_action) = flag_action(input)
                .with_context(|| format!("at `{input}`: failed to parse flag action"))?;

            result.push(Action::Flag(action));
            input = after_action.trim_start();
        } else {
            let (action, after_action) = var_action(input)
                .with_context(|| format!("at `{input}`: failed to parse var action"))?;

            result.push(Action::Var(action));
            input = after_action.trim_start();
        }
    }

    if result.is_empty() {
        anyhow::bail!("expected at least one action");
    }

    Ok(result)
}

/// Parses a [`Matcher`] and returns it together with the rest of the input.
fn matcher<'a>(
    input: &'a str,
    frame_offset: FrameOffset,
    regex_cache: &mut RegexCache,
) -> anyhow::Result<(Matcher, &'a str)> {
    let input = input.trim_start();

    let (negated, before_name) = if let Some(rest) = input.strip_prefix('!') {
        (true, rest)
    } else {
        (false, input)
    };

    let (name, after_name) = ident(before_name)
        .with_context(|| format!("at `{before_name}`: failed to parse matcher name"))?;

    let before_arg = expect(after_name, ":")?;

    let (arg, rest) = argument(before_arg)
        .with_context(|| format!("at `{before_arg}`: failed to parse matcher argument"))?;

    let m = Matcher::new(negated, name, &arg, frame_offset, regex_cache)?;
    Ok((m, rest))
}

/// Parses the caller matcher in a rule and returns it together with the rest of the input.
///
/// A caller matcher is defined as `[ <matcher> ] |`.
/// NB: This function assumes that the leading `[` has already been consumed!
fn caller_matcher<'a>(
    input: &'a str,
    regex_cache: &mut RegexCache,
) -> anyhow::Result<(Matcher, &'a str)> {
    let (matcher, rest) = matcher(input, FrameOffset::Caller, regex_cache)?;

    let rest = rest.trim_start();
    let rest = expect(rest, "]")?;

    let rest = rest.trim_start();
    let rest = expect(rest, "|")?;

    Ok((matcher, rest))
}

/// Parses the callee matcher in a rule and returns it together with the rest of the input.
///
/// A callee matcher is defined as `| [ <matcher> ] `.
/// NB: This function assumes that the leading `|` has already been consumed!
fn callee_matcher<'a>(
    input: &'a str,
    regex_cache: &mut RegexCache,
) -> anyhow::Result<(Matcher, &'a str)> {
    let rest = input.trim_start();
    let rest = expect(rest, "[")?;

    let (matcher, rest) = matcher(rest, FrameOffset::Callee, regex_cache)?;

    let rest = rest.trim_start();
    let rest = expect(rest, "]")?;

    Ok((matcher, rest))
}

/// Parses a sequence of [`Matchers`](Matcher) and returns it
/// together with the rest of the input.
///
/// The sequence must contain at least one matcher.
fn matchers<'a>(
    input: &'a str,
    regex_cache: &mut RegexCache,
) -> anyhow::Result<(Vec<Matcher>, &'a str)> {
    let mut input = input.trim_start();

    let mut result = Vec::new();

    // A `[` at the start means we have a caller matcher
    if let Some(rest) = input.strip_prefix('[') {
        let (caller_matcher, rest) = caller_matcher(rest, regex_cache)
            .with_context(|| format!("at `{input}`: failed to parse caller matcher"))?;

        result.push(caller_matcher);

        input = rest.trim_start()
    }

    // Keep track of whether we've parsed at least one matcher
    let mut parsed = false;

    while MATCHER_LOOKAHEAD
        .iter()
        .any(|prefix| input.starts_with(prefix))
    {
        let (m, rest) = matcher(input, FrameOffset::None, regex_cache)
            .with_context(|| format!("at `{input}`: failed to parse matcher"))?;
        result.push(m);
        input = rest.trim_start();
        parsed = true;
    }

    if !parsed {
        anyhow::bail!("at `{input}`: expected at least one matcher");
    }

    // A `|` after the main list of matchers means we have a callee matcher.
    if let Some(rest) = input.strip_prefix('|') {
        let (callee_matcher, rest) = callee_matcher(rest, regex_cache)
            .with_context(|| format!("at `{input}`: failed to parse callee matcher"))?;

        result.push(callee_matcher);
        input = rest;
    }

    Ok((result, input))
}

/// Parses a [`Rule`] from its string representation.
///
/// `regex_cache` is used to memoize the construction of regexes.
pub fn parse_rule(input: &str, regex_cache: &mut RegexCache) -> anyhow::Result<Rule> {
    let (matchers, after_matchers) = matchers(input, regex_cache)
        .with_context(|| format!("at `{input}`: failed to parse matchers"))?;
    let actions = actions(after_matchers)
        .with_context(|| format!("at `{after_matchers}`: failed to parse actions"))?;

    Ok(Rule::new(matchers, actions))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::enhancers::config_structure::EncodedMatcher;
    use crate::enhancers::Frame;

    use super::*;

    #[test]
    fn parse_objc_matcher() {
        let rule = parse_rule("stack.function:-[* -app", &mut RegexCache::default()).unwrap();

        let frames = &[Frame::from_test(
            &json!({"function": "-[UIApplication sendAction:to:from:forEvent:] "}),
            "native",
        )];
        assert!(!rule.matches_frame(frames, 0));

        let matcher: EncodedMatcher = serde_json::from_str(r#""f-[*""#).unwrap();
        let matcher = matcher.into_matcher(&mut Default::default()).unwrap();
        match matcher {
            Matcher::Frame(frame) => {
                assert!(!frame.matches_frame(frames, 0));
            }
            Matcher::Exception(_) => unreachable!(),
        }

        let _rule = parse_rule("stack.module:[foo:bar/* -app", &mut Default::default()).unwrap();
    }

    #[test]
    fn invalid_app_matcher() {
        let rule = parse_rule(
            "app://../../src/some-file.ts -group -app",
            &mut Default::default(),
        )
        .unwrap();

        let frames = &[
            Frame::from_test(&json!({}), "native"),
            Frame::from_test(&json!({"in_app": true}), "native"),
            Frame::from_test(&json!({"in_app": false}), "native"),
        ];
        assert!(!rule.matches_frame(frames, 0));
        assert!(!rule.matches_frame(frames, 1));
        assert!(!rule.matches_frame(frames, 2));
    }
}
