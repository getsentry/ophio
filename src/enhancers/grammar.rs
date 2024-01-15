/*
https://github.com/getsentry/sentry/blob/e5c5e56d176d96081ce4b25424e6ec7d3ba17cff/src/sentry/grouping/enhancer/__init__.py#L42-L79

enhancements = line+

line = _ (comment / rule / empty) newline?

rule = _ matchers actions


matchers         = caller_matcher? frame_matcher+ callee_matcher?
frame_matcher    = _ negation? matcher_type sep argument
matcher_type     = ident / quoted_ident
caller_matcher   = _ "[" _ frame_matcher _ "]" _ "|"
callee_matcher   = _ "|" _ "[" _ frame_matcher _ "]"

actions          = action+
action           = flag_action / var_action
var_action       = _ var_name _ "=" _ ident
var_name         = "max-frames" / "min-frames" / "invert-stacktrace" / "category"
flag_action      = _ range? flag flag_action_name
flag_action_name = "group" / "app" / "prefix" / "sentinel"
flag             = "+" / "-"
range            = "^" / "v"

ident            = ~r"[a-zA-Z0-9_\.-]+"
quoted_ident     = ~r"\"([a-zA-Z0-9_\.:-]+)\""

comment          = ~r"#[^\r\n]*"

argument         = quoted / unquoted
quoted           = ~r'"([^"\\]*(?:\\.[^"\\]*)*)"'
unquoted         = ~r"\S+"

sep      = ":"
space    = " "
empty    = ""
negation = "!"
newline  = ~r"[\r\n]"
_        = space*

*/

use smol_str::SmolStr;

pub use nom::parse_enhancers;

#[derive(Debug)]
pub struct RawMatcher {
    pub negation: bool,
    pub ty: SmolStr,
    pub argument: SmolStr,
}

#[derive(Debug)]
pub struct RawMatchers {
    pub caller_matcher: Option<RawMatcher>,
    pub matchers: Vec<RawMatcher>,
    pub callee_matcher: Option<RawMatcher>,
}
#[derive(Debug)]
pub enum RawAction {
    Var(SmolStr, SmolStr),
    Flag(Option<char>, char, SmolStr),
}
#[derive(Debug)]
pub struct RawRule {
    pub matchers: RawMatchers,
    pub actions: Vec<RawAction>,
}

mod nom {
    use nom::branch::alt;
    use nom::bytes::complete::{tag, take_while1};
    use nom::character::complete::{char, one_of, space0};
    use nom::combinator::{all_consuming, opt};
    use nom::multi::many1;
    use nom::sequence::{delimited, pair, preceded, tuple};
    use nom::{Finish, IResult, Parser};

    use super::{RawAction, RawMatcher, RawMatchers, RawRule};

    fn ident(input: &str) -> IResult<&str, &str> {
        take_while1(|c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))(input)
    }

    fn frame_matcher(input: &str) -> IResult<&str, RawMatcher> {
        /*let (input, is_negation) = opt(tag("!"))(input)?;
        let (input, matcher_type) = alt(ident, quoted_ident)(input)?;
        let (input, _) = tag(":")(input)?;
        let (input, argument) = alt(quoted, unquoted)(input)?;*/
        let input = input.trim_start();

        let quoted_ident = delimited(
            char('"'),
            take_while1(|c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':' | '-')),
            char('"'),
        );
        let matcher_type = alt((ident, quoted_ident));

        let unquoted = take_while1(|c: char| !c.is_ascii_whitespace());
        // let quoted = delimited(
        //     char('"'),
        //     escaped_transform(
        //         alpha1,
        //         '\\',
        //         alt((
        //             value("\\", tag("\\")),
        //             value("\"", tag("\"")),
        //             value("\n", tag("n")),
        //         )),
        //     ),
        //     char('"'),
        // );
        //let argument = alt((quoted, unquoted));
        let argument = unquoted;

        let (input, (is_negation, matcher_type, _, argument)) =
            tuple((opt(char('!')), matcher_type, char(':'), argument))(input)?;

        let matcher = RawMatcher {
            negation: is_negation.is_some(),
            ty: matcher_type.into(),
            argument: argument.into(),
        };
        Ok((input, matcher))
    }

    fn matchers(input: &str) -> IResult<&str, RawMatchers> {
        let input = input.trim_start();

        let caller_matcher = tuple((
            space0,
            char('['),
            space0,
            frame_matcher,
            space0,
            char(']'),
            space0,
            char('|'),
        ));
        let callee_matcher = tuple((
            space0,
            char('|'),
            space0,
            char('['),
            space0,
            frame_matcher,
            space0,
            char(']'),
        ));

        let mut matchers = tuple((
            opt(caller_matcher),
            many1(frame_matcher),
            opt(callee_matcher),
        ));

        let (input, (caller_matcher, matchers, callee_matcher)) = matchers(input)?;

        let caller_matcher = caller_matcher.map(|(_, _, _, m, _, _, _, _)| m);
        let callee_matcher = callee_matcher.map(|(_, _, _, _, _, m, _, _)| m);

        let matchers = RawMatchers {
            caller_matcher,
            matchers,
            callee_matcher,
        };
        Ok((input, matchers))
    }

    fn actions(input: &str) -> IResult<&str, Vec<RawAction>> {
        let var_name = alt((
            tag("max-frames"),
            tag("min-frames"),
            tag("invert-stacktrace"),
            tag("category"),
        ));
        let var_action = tuple((var_name, space0, char('='), space0, ident))
            .map(|(var_name, _, _, _, ident)| RawAction::Var(var_name.into(), ident.into()));

        let flag_name = alt((tag("group"), tag("app"), tag("prefix"), tag("sentinel")));
        let flag_action = tuple((opt(one_of("^v")), one_of("+-"), flag_name)).map(
            |(range, flag, flag_name): (_, _, &str)| RawAction::Flag(range, flag, flag_name.into()),
        );

        let action = preceded(space0, alt((flag_action, var_action)));

        let (input, actions) = many1(action)(input)?;

        Ok((input, actions))
    }

    fn rule(input: &str) -> anyhow::Result<RawRule> {
        let (_input, (matchers, actions)) = all_consuming(pair(matchers, actions))(input)
            .finish()
            .map_err(|e| anyhow::Error::msg(e.to_string()))?;

        Ok(RawRule { matchers, actions })
    }

    pub fn parse_enhancers(input: &str) -> anyhow::Result<Vec<RawRule>> {
        let mut rules = vec![];

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let rule = rule(line)?;
            rules.push(rule);
        }

        Ok(rules)
    }
}

mod chumsky {
    use chumsky::prelude::*;

    use super::{RawAction, RawMatchers, RawRule};

    fn ident() -> impl Parser<char, String, Error = Simple<char>> {
        filter(|c: &char| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
            .repeated()
            .at_least(1)
            .collect()
    }

    fn frame_matcher() -> impl Parser<char, super::RawMatcher, Error = Simple<char>> {
        let quoted_ident =
            filter(|c: &char| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':' | '-'))
                .repeated()
                .at_least(1)
                .collect()
                .delimited_by(just('"'), just('"'));
        let matcher_type = ident().or(quoted_ident);

        let unquoted = filter(|c: &char| !c.is_ascii_whitespace())
            .repeated()
            .at_least(1)
            .collect::<String>();
        let argument = unquoted;

        just('!')
            .or_not()
            .then(matcher_type)
            .then_ignore(just(':'))
            .then(argument)
            .map(|((negation, ty), argument)| super::RawMatcher {
                negation: negation.is_some(),
                ty: ty.into(),
                argument: argument.into(),
            })
            .padded()
    }

    fn matchers() -> impl Parser<char, super::RawMatchers, Error = Simple<char>> {
        let caller_matcher = just('[')
            .padded()
            .ignore_then(frame_matcher())
            .then_ignore(just(']').padded())
            .then_ignore(just('|'));
        let callee_matcher = just('|')
            .padded()
            .ignore_then(just('['))
            .ignore_then(frame_matcher().padded())
            .then_ignore(just(']'));

        caller_matcher
            .or_not()
            .then(frame_matcher().repeated())
            .then(callee_matcher.or_not())
            .map(|((caller_matcher, matchers), callee_matcher)| RawMatchers {
                caller_matcher,
                matchers,
                callee_matcher,
            })
    }

    fn actions() -> impl Parser<char, Vec<RawAction>, Error = Simple<char>> {
        let var_name = choice((
            just("max-frames"),
            just("min-frames"),
            just("invert-stacktrace"),
            just("category"),
        ));
        let var_action = var_name
            .then_ignore(just('=').padded())
            .then(ident())
            .map(|(var_name, ident)| RawAction::Var(var_name.into(), ident.into()));

        let flag_name = choice((just("group"), just("app"), just("prefix"), just("sentinel")));

        let flag_action = one_of("^v")
            .or_not()
            .then(one_of("+-"))
            .then(flag_name)
            .map(|((range, flag), flag_name)| RawAction::Flag(range, flag, flag_name.into()));

        choice((flag_action, var_action))
            .padded()
            .repeated()
            .at_least(1)
    }

    fn rule(input: &str) -> anyhow::Result<RawRule> {
        let (matchers, actions) = matchers()
            .then(actions())
            .parse(input)
            .map_err(|e| anyhow::Error::msg(e.first().unwrap().to_string()))?;

        Ok(RawRule { matchers, actions })
    }

    pub fn parse_enhancers(input: &str) -> anyhow::Result<Vec<RawRule>> {
        let mut rules = vec![];

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let rule = rule(line)?;
            rules.push(rule);
        }

        Ok(rules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_example() {
        let input = r#"
# This is a config
path:*/code/game/whatever/*                     +app
function:panic_handler                          ^-group -group
function:ThreadStartWin32                       v-group
function:ThreadStartLinux                       v-group
function:ThreadStartMac                         v-group
family:native module:std::*                     -app
module:core::*                                  -app
family:javascript path:*/test.js                -app
family:javascript app:1 path:*/test.js          -app
family:native                                   max-frames=3
"#;

        dbg!(nom::parse_enhancers(input));

        dbg!(chumsky::parse_enhancers(input));
    }
}
