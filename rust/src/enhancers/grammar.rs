//! Parse enhancement rules from the string representation.
//!
//! The parser is made using the `nom` parser combinator library.
//! The grammar was adapted to `nom` from:
//! <https://github.com/getsentry/sentry/blob/e5c5e56d176d96081ce4b25424e6ec7d3ba17cff/src/sentry/grouping/enhancer/__init__.py#L42-L79>

// TODO:
// - we should probably support better Error handling
// - quoted identifiers/arguments should properly support escapes, etc

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

fn expect(input: &mut &str, pat: &str) -> anyhow::Result<()> {
    *input = input
        .strip_prefix(pat)
        .ok_or_else(|| anyhow!("at `{input}`: expected `{pat}`"))?;
    Ok(())
}

fn skip_ws(input: &mut &str) {
    *input = input.trim_start();
}

fn bool(input: &str) -> anyhow::Result<bool> {
    match input {
        "1" | "yes" | "true" => Ok(true),
        "0" | "no" | "false" => Ok(false),
        _ => anyhow::bail!("at `{input}`: invalid boolean value"),
    }
}

fn ident<'a>(input: &mut &'a str) -> anyhow::Result<&'a str> {
    let Some(end) =
        input.find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-')))
    else {
        let res = *input;
        *input = "";
        return Ok(res);
    };

    if end == 0 {
        anyhow::bail!("at `{input}`: invalid identifier");
    }

    let (ident, rest) = input.split_at(end);
    *input = rest;
    Ok(ident)
}

fn argument<'a>(input: &mut &'a str) -> anyhow::Result<&'a str> {
    if let Some(rest) = input.strip_prefix('"') {
        let end = rest
            .find('"')
            .ok_or_else(|| anyhow!("at `{input}`: unclosed `\"`"))?;
        let result = &rest[..end];
        *input = rest.get(end + 1..).unwrap_or_default();
        Ok(result)
    } else {
        match input.find(|c: char| c.is_ascii_whitespace()) {
            None => {
                let result = *input;
                *input = "";
                Ok(result)
            }

            Some(end) => {
                let (result, rest) = input.split_at(end);
                *input = rest;
                Ok(result)
            }
        }
    }
}

fn var_action(input: &mut &str) -> anyhow::Result<VarAction> {
    skip_ws(input);

    let starting_input = *input;

    let lhs = ident(input).with_context(|| format!("at `{input}`: expected variable name"))?;

    skip_ws(input);

    expect(input, "=")?;

    skip_ws(input);

    let rhs = ident(input).with_context(|| format!("at `{input}`: expected value for variable"))?;

    match lhs {
        "max-frames" => {
            let n = rhs
                .parse()
                .with_context(|| format!("at `{rhs}`: failed to parse rhs of `max-frames`"))?;
            Ok(VarAction::MaxFrames(n))
        }

        "min-frames" => {
            let n = rhs
                .parse()
                .with_context(|| format!("at `{rhs}`: failed to parse rhs of `min-frames`"))?;
            Ok(VarAction::MinFrames(n))
        }

        "invert-stacktrace" => {
            let b = bool(rhs).with_context(|| {
                format!("at `{rhs}`: failed to parse rhs of `invert-stacktrace`")
            })?;
            Ok(VarAction::InvertStacktrace(b))
        }

        "category" => Ok(VarAction::Category(rhs.into())),

        _ => Err(anyhow!(
            "at `{starting_input}`: invalid variable name `{lhs}`"
        )),
    }
}

fn flag_action(input: &mut &str) -> anyhow::Result<FlagAction> {
    skip_ws(input);

    let range = if let Some(rest) = input.strip_prefix('^') {
        *input = rest;
        Some(Range::Up)
    } else if let Some(rest) = input.strip_prefix('v') {
        *input = rest;
        Some(Range::Down)
    } else {
        None
    };

    let flag = if let Some(rest) = input.strip_prefix('+') {
        *input = rest;
        true
    } else if let Some(rest) = input.strip_prefix('-') {
        *input = rest;
        false
    } else {
        anyhow::bail!("at `{input}`: expected flag value");
    };

    let before_name = *input;
    let name = ident(input).with_context(|| format!("at `{input}`: expected flag name"))?;

    let ty = match name {
        "app" => FlagActionType::App,
        "group" => FlagActionType::Group,
        "prefix" => FlagActionType::Prefix,
        "sentinel" => FlagActionType::Sentinel,
        _ => anyhow::bail!("at `{before_name}`: invalid flag name `{name}`"),
    };

    Ok(FlagAction { flag, ty, range })
}

fn actions(input: &mut &str) -> anyhow::Result<Vec<Action>> {
    let mut result = Vec::new();
    skip_ws(input);

    while !input.is_empty() && !input.starts_with('#') {
        let starting_input = *input;

        if input.starts_with(['v', '^', '+', '-']) {
            let action = flag_action(input)
                .with_context(|| format!("at `{starting_input}`: failed to parse flag action"))?;
            result.push(Action::Flag(action));
        } else {
            let action = var_action(input)
                .with_context(|| format!("at `{starting_input}`: failed to parse var action"))?;
            result.push(Action::Var(action));
        }

        skip_ws(input);
    }

    if result.is_empty() {
        anyhow::bail!("expected at least one action");
    }

    Ok(result)
}

fn matcher(
    input: &mut &str,
    frame_offset: FrameOffset,
    regex_cache: &mut RegexCache,
) -> anyhow::Result<Matcher> {
    skip_ws(input);

    let negated = if let Some(rest) = input.strip_prefix('!') {
        *input = rest;
        true
    } else {
        false
    };

    let name =
        ident(input).with_context(|| format!("at `{input}`: failed to parse matcher name"))?;

    expect(input, ":")?;

    let arg = argument(input)
        .with_context(|| format!("at `{input}`: failed to parse matcher argument"))?;

    // TODO: support even more escapes
    let unescaped = arg.replace("\\\\", "\\");
    Matcher::new(negated, name, &unescaped, frame_offset, regex_cache)
}

fn matchers(input: &mut &str, regex_cache: &mut RegexCache) -> anyhow::Result<Vec<Matcher>> {
    let mut result = Vec::new();
    skip_ws(input);

    if let Some(rest) = input.strip_prefix('[') {
        *input = rest;
        let caller_matcher = matcher(input, FrameOffset::Caller, regex_cache)
            .with_context(|| format!("at `{rest}`: failed to parse caller matcher"))?;
        skip_ws(input);
        dbg!(&input);
        expect(input, "]")
            .with_context(|| format!("at `{input}`: failed to parse caller matcher"))?;
        dbg!(&input);
        skip_ws(input);
        expect(input, "|")
            .with_context(|| format!("at `{input}`: failed to parse caller matcher"))?;
        dbg!(&input);

        result.push(caller_matcher);
    }

    let mut parsed = false;

    skip_ws(input);
    while MATCHER_LOOKAHEAD
        .iter()
        .any(|prefix| input.starts_with(prefix))
    {
        let starting_input = *input;
        let m = matcher(input, FrameOffset::None, regex_cache)
            .with_context(|| format!("at `{starting_input}`: failed to parse matcher"))?;
        result.push(m);
        skip_ws(input);
        parsed = true;
    }

    if !parsed {
        anyhow::bail!("at `{input}`: expected at least one matcher");
    }

    if let Some(rest) = input.strip_prefix('|') {
        *input = rest.trim_start();
        expect(input, "[")
            .with_context(|| format!("at `{input}`: failed to parse callee matcher"))?;
        let callee_matcher = matcher(input, FrameOffset::Callee, regex_cache)
            .with_context(|| format!("at `{input}`: failed to parse callee matcher"))?;
        skip_ws(input);
        expect(input, "]")
            .with_context(|| format!("at `{input}`: failed to parse callee matcher"))?;

        result.push(callee_matcher);
    }

    Ok(result)
}

pub fn parse_rule(mut input: &str, regex_cache: &mut RegexCache) -> anyhow::Result<Rule> {
    let matchers = matchers(&mut input, regex_cache)?;
    let actions = actions(&mut input)?;

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
