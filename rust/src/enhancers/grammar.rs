//! Parse enhancement rules from the string representation.
//!
//! The parser is made using the `nom` parser combinator library.
//! The grammar was adapted to `nom` from:
//! <https://github.com/getsentry/sentry/blob/e5c5e56d176d96081ce4b25424e6ec7d3ba17cff/src/sentry/grouping/enhancer/__init__.py#L42-L79>

// TODO:
// - we should probably support better Error handling
// - quoted identifiers/arguments should properly support escapes, etc

use std::borrow::Cow;

use anyhow::{anyhow, Context};

use super::actions::{Action, FlagAction, FlagActionType, Range, VarAction};
use super::matchers::{FrameOffset, Matcher};
use super::rules::Rule;
use super::RegexCache;

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

fn expect<'a>(input: &'a str, pat: &str) -> anyhow::Result<&'a str> {
    input
        .strip_prefix(pat)
        .ok_or_else(|| anyhow!("at `{input}`: expected `{pat}`"))
}

fn bool(input: &str) -> anyhow::Result<bool> {
    match input {
        "1" | "yes" | "true" => Ok(true),
        "0" | "no" | "false" => Ok(false),
        _ => anyhow::bail!("at `{input}`: invalid boolean value"),
    }
}

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

fn flag_action(input: &str) -> anyhow::Result<(FlagAction, &str)> {
    let input = input.trim_start();

    let (range, after_range) = if let Some(rest) = input.strip_prefix('^') {
        (Some(Range::Up), rest)
    } else if let Some(rest) = input.strip_prefix('v') {
        (Some(Range::Up), rest)
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

fn actions(input: &str) -> anyhow::Result<Vec<Action>> {
    let mut input = input.trim_start();

    let mut result = Vec::new();

    while !input.is_empty() && !input.starts_with('#') {
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

fn matchers<'a>(
    input: &'a str,
    regex_cache: &mut RegexCache,
) -> anyhow::Result<(Vec<Matcher>, &'a str)> {
    let input = input.trim_start();

    let mut result = Vec::new();

    let mut input = if let Some(rest) = input.strip_prefix('[') {
        let (caller_matcher, rest) = matcher(rest, FrameOffset::Caller, regex_cache)
            .with_context(|| format!("at `{rest}`: failed to parse caller matcher"))?;
        let rest = rest.trim_start();
        let rest = expect(rest, "]")
            .with_context(|| format!("at `{rest}`: failed to parse caller matcher"))?;
        let rest = rest.trim_start();
        let rest = expect(rest, "|")
            .with_context(|| format!("at `{rest}`: failed to parse caller matcher"))?;

        result.push(caller_matcher);

        rest.trim_start()
    } else {
        input
    };

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

    let rest = if let Some(rest) = input.strip_prefix('|') {
        let rest = rest.trim_start();
        let rest = expect(rest, "[")
            .with_context(|| format!("at `{rest}`: failed to parse callee matcher"))?;
        let (callee_matcher, rest) = matcher(rest, FrameOffset::Callee, regex_cache)
            .with_context(|| format!("at `{rest}`: failed to parse callee matcher"))?;
        let rest = rest.trim_start();
        let rest = expect(rest, "]")
            .with_context(|| format!("at `{rest}`: failed to parse callee matcher"))?;

        result.push(callee_matcher);
        rest
    } else {
        input
    };

    Ok((result, rest))
}

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
