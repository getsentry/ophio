use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use globset::GlobBuilder;
use regex::bytes::{Regex, RegexBuilder};
use smol_str::SmolStr;

#[derive(Debug, Clone)]
pub struct Frame {
    // TODO:
    fields: HashMap<&'static str, &'static str>,
}

impl Frame {
    fn get_field(&self, field: &str) -> Option<&str> {
        self.fields.get(field).copied()
    }
}

/*
MATCHERS = {
    # discover field names
    # fingerprinting specific fields


        if value in ("1", "yes", "true"):
            return True
        elif value in ("0", "no", "false"):
            return False
*/

fn boolean_value(value: &str) -> bool {
    matches!(value, "1" | "yes" | "true")
}

fn translate_pattern(pat: &str, is_path_matcher: bool) -> anyhow::Result<Regex> {
    let pat = if is_path_matcher {
        pat.replace('\\', "/")
    } else {
        pat.into()
    };
    let mut builder = GlobBuilder::new(&pat);
    builder.literal_separator(is_path_matcher);
    builder.case_insensitive(true);
    let glob = builder.build()?;
    Ok(RegexBuilder::new(glob.regex()).build()?)
}

#[derive(Debug, Clone)]
pub enum FrameOrExceptionMatcher<F, E> {
    Frame(F),
    Exception(E),
}

pub type Matcher = FrameOrExceptionMatcher<Arc<dyn FrameMatcher>, Arc<dyn ExceptionMatcher>>;

pub fn get_matcher(negated: bool, matcher_type: &str, argument: &str) -> anyhow::Result<Matcher> {
    // TODO: cache based on (negated, matcher_type, argument)
    let matcher = match matcher_type {
        // Field matchers
        "stack.module" | "module" => FrameOrExceptionMatcher::Frame(frame_matcher(
            negated,
            FrameFieldMatch::new("module", argument)?,
        )),
        "stack.function" | "function" => FrameOrExceptionMatcher::Frame(frame_matcher(
            negated,
            FrameFieldMatch::new("function", argument)?,
        )),
        "category" => FrameOrExceptionMatcher::Frame(frame_matcher(
            negated,
            FrameFieldMatch::new("category", argument)?,
        )),

        // Path matchers
        "stack.abs_path" | "path" => FrameOrExceptionMatcher::Frame(frame_matcher(
            negated,
            PathLikeMatch::new("path", argument)?,
        )),
        "stack.package" | "package" => FrameOrExceptionMatcher::Frame(frame_matcher(
            negated,
            PathLikeMatch::new("package", argument)?,
        )),

        // Family matcher
        "family" => {
            FrameOrExceptionMatcher::Frame(frame_matcher(negated, FamilyMatch::new(argument)))
        }

        // InApp matcher
        "app" => FrameOrExceptionMatcher::Frame(frame_matcher(negated, InAppMatch::new(argument)?)),

        // Exception matchers
        "error.type" | "type" => FrameOrExceptionMatcher::Exception(exception_matcher(
            negated,
            ExceptionTypeMatch::new(argument)?,
        )),
        "error.value" | "value" => FrameOrExceptionMatcher::Exception(exception_matcher(
            negated,
            ExceptionValueMatch::new(argument)?,
        )),
        "error.mechanism" | "mechanism" => FrameOrExceptionMatcher::Exception(exception_matcher(
            negated,
            ExceptionMechanismMatch::new(argument)?,
        )),

        matcher_type => anyhow::bail!("Unknown matcher `{matcher_type}`"),
    };

    Ok(matcher)
}

#[derive(Debug, Clone, Default)]
struct ExceptionData {
    ty: Option<String>,
    value: Option<String>,
    mechanism: Option<String>,
}

pub trait FrameMatcher {
    fn matches_frame(&self, frames: &[Frame], idx: usize) -> bool;
}

trait SimpleFieldMatcher {
    fn field(&self) -> &str;
    fn matches_value(&self, value: &str) -> bool;
}

impl<S: SimpleFieldMatcher> FrameMatcher for S {
    fn matches_frame(&self, frames: &[Frame], idx: usize) -> bool {
        let Some(frame) = frames.get(idx) else {
            return false;
        };

        let Some(value) = frame.get_field(self.field()) else {
            return false;
        };

        self.matches_value(value)
    }
}

#[derive(Debug, Clone)]
struct NegationWrapper<M> {
    negated: bool,
    inner: M,
}

impl<M: FrameMatcher> FrameMatcher for NegationWrapper<M> {
    fn matches_frame(&self, frames: &[Frame], idx: usize) -> bool {
        self.negated ^ self.inner.matches_frame(frames, idx)
    }
}

fn frame_matcher<M: FrameMatcher + 'static>(negated: bool, matcher: M) -> Arc<dyn FrameMatcher> {
    Arc::new(NegationWrapper {
        negated,
        inner: matcher,
    })
}

#[derive(Debug, Clone)]
struct FrameFieldMatch {
    field: &'static str, // function, module, category
    pattern: Regex,
}

impl FrameFieldMatch {
    fn new(field: &'static str, pattern: &str) -> anyhow::Result<Self> {
        let pattern = translate_pattern(pattern, false)?;

        Ok(Self { field, pattern })
    }
}

impl SimpleFieldMatcher for FrameFieldMatch {
    fn field(&self) -> &str {
        self.field
    }
    fn matches_value(&self, value: &str) -> bool {
        self.pattern.is_match(value.as_bytes())
    }
}

#[derive(Debug, Clone)]
struct PathLikeMatch {
    field: &'static str, // package, path
    pattern: Regex,      // translate_pattern(true)
}

impl PathLikeMatch {
    fn new(field: &'static str, pattern: &str) -> anyhow::Result<Self> {
        let pattern = translate_pattern(pattern, true)?;

        Ok(Self { field, pattern })
    }
}

impl SimpleFieldMatcher for PathLikeMatch {
    fn field(&self) -> &str {
        self.field
    }

    fn matches_value(&self, value: &str) -> bool {
        // normalize path:
        let mut value = value.replace('\\', "/");

        if self.pattern.is_match(value.as_bytes()) {
            return true;
        }

        if !value.starts_with('/') {
            value.insert(0, '/');
            return self.pattern.is_match(value.as_bytes());
        }

        false
    }
}

#[derive(Debug, Clone)]
struct FamilyMatch {
    families: HashSet<SmolStr>,
}

impl FamilyMatch {
    fn new(families: &str) -> Self {
        let families = families.split(',').map(SmolStr::from).collect();

        Self { families }
    }
}

impl SimpleFieldMatcher for FamilyMatch {
    fn field(&self) -> &str {
        "family"
    }

    fn matches_value(&self, value: &str) -> bool {
        self.families.contains("all") || self.families.contains(value)
    }
}

#[derive(Debug, Clone)]
struct InAppMatch {
    expected: bool,
}

impl InAppMatch {
    fn new(expected: &str) -> anyhow::Result<Self> {
        match expected {
            "1" | "true" | "yes" => Ok(Self { expected: true }),
            "0" | "false" | "no" => Ok(Self { expected: false }),
            _ => Err(anyhow::anyhow!("Invalid value for `app`: `{expected}`")),
        }
    }
}

impl SimpleFieldMatcher for InAppMatch {
    fn field(&self) -> &str {
        "in_app"
    }

    fn matches_value(&self, value: &str) -> bool {
        // TODO!!!
        boolean_value(value) == self.expected
    }
}

pub trait ExceptionMatcher {
    fn matches_exception(&self, exception_data: &ExceptionData) -> bool;
}

struct ExceptionTypeMatch {
    pattern: Regex,
}

impl ExceptionTypeMatch {
    fn new(pattern: &str) -> anyhow::Result<Self> {
        let pattern = translate_pattern(pattern, false)?;
        Ok(Self { pattern })
    }
}

impl ExceptionMatcher for ExceptionTypeMatch {
    fn matches_exception(&self, exception_data: &ExceptionData) -> bool {
        let ty = exception_data.ty.as_deref().unwrap_or("<unknown>");
        self.pattern.is_match(ty.as_bytes())
    }
}

struct ExceptionValueMatch {
    pattern: Regex,
}

impl ExceptionValueMatch {
    fn new(pattern: &str) -> anyhow::Result<Self> {
        let pattern = translate_pattern(pattern, false)?;
        Ok(Self { pattern })
    }
}

impl ExceptionMatcher for ExceptionValueMatch {
    fn matches_exception(&self, exception_data: &ExceptionData) -> bool {
        let value = exception_data.value.as_deref().unwrap_or("<unknown>");
        self.pattern.is_match(value.as_bytes())
    }
}

struct ExceptionMechanismMatch {
    pattern: Regex,
}

impl ExceptionMechanismMatch {
    fn new(pattern: &str) -> anyhow::Result<Self> {
        let pattern = translate_pattern(pattern, false)?;
        Ok(Self { pattern })
    }
}

impl ExceptionMatcher for ExceptionMechanismMatch {
    fn matches_exception(&self, exception_data: &ExceptionData) -> bool {
        let mechanism = exception_data.mechanism.as_deref().unwrap_or("<unknown>");
        self.pattern.is_match(mechanism.as_bytes())
    }
}

impl<M: ExceptionMatcher> ExceptionMatcher for NegationWrapper<M> {
    fn matches_exception(&self, exception_data: &ExceptionData) -> bool {
        self.negated ^ self.inner.matches_exception(exception_data)
    }
}

fn exception_matcher<M: ExceptionMatcher + 'static>(
    negated: bool,
    matcher: M,
) -> Arc<dyn ExceptionMatcher> {
    Arc::new(NegationWrapper {
        negated,
        inner: matcher,
    })
}

#[cfg(test)]
mod tests {
    use crate::enhancers::grammar::parse_enhancers;

    use super::*;

    fn create_matcher(rules: &str) -> impl Fn(Frame) -> bool {
        let rules = parse_enhancers(rules).unwrap();
        let rule = &rules[0];
        let matchers: Vec<_> = rule
            .matchers
            .matchers
            .iter()
            .map(|matcher| get_matcher(matcher.negation, &matcher.ty, &matcher.argument).unwrap())
            .filter_map(|m| match m {
                FrameOrExceptionMatcher::Frame(m) => Some(m),
                FrameOrExceptionMatcher::Exception(_) => None,
            })
            .collect();

        move |frame: Frame| {
            let frames = &[frame];
            matchers
                .iter()
                .all(|matcher| matcher.matches_frame(frames, 0))
        }
    }

    #[test]
    fn path_matching() {
        let matcher = create_matcher("path:**/test.js              +app");

        assert!(matcher(Frame {
            fields: [("path", "http://example.com/foo/test.js"),].into()
        }));

        assert!(!matcher(Frame {
            fields: [("path", "http://example.com/foo/bar.js"),].into()
        }));

        assert!(matcher(Frame {
            fields: [("path", "http://example.com/foo/test.js")].into()
        }));

        assert!(!matcher(Frame {
            fields: [("path", "/foo/bar.js")].into()
        }));

        assert!(matcher(Frame {
            fields: [("path", "http://example.com/foo/TEST.js")].into()
        }));

        assert!(!matcher(Frame {
            fields: [("path", "http://example.com/foo/bar.js")].into()
        }));
    }

    #[test]
    fn family_matching() {
        let js_matcher = create_matcher("family:javascript path:**/test.js              +app");
        let native_matcher = create_matcher("family:native function:std::*                  -app");

        assert!(js_matcher(Frame {
            fields: [
                ("path", "http://example.com/foo/TEST.js"),
                ("family", "javascript")
            ]
            .into()
        }));
        assert!(!js_matcher(Frame {
            fields: [
                ("path", "http://example.com/foo/TEST.js"),
                ("family", "native")
            ]
            .into()
        }));

        assert!(!native_matcher(Frame {
            fields: [
                ("path", "http://example.com/foo/TEST.js"),
                ("function", "std::whatever"),
                ("family", "javascript")
            ]
            .into()
        }));
        assert!(native_matcher(Frame {
            fields: [("function", "std::whatever"), ("family", "native")].into()
        }));
    }

    #[test]
    fn app_matching() {
        let yes_matcher = create_matcher("family:javascript path:**/test.js app:yes       +app");
        let no_matcher = create_matcher("family:native path:**/test.c app:no            -group");

        // TODO:
        assert!(yes_matcher(Frame {
            fields: [
                ("path", "http://example.com/foo/TEST.js"),
                ("family", "javascript"),
                ("in_app", "true")
            ]
            .into()
        }));
        assert!(!yes_matcher(Frame {
            fields: [
                ("path", "http://example.com/foo/TEST.js"),
                ("family", "javascript"),
                ("in_app", "false")
            ]
            .into()
        }));
        assert!(no_matcher(Frame {
            fields: [
                ("path", "/test.c"),
                ("family", "native"),
                ("in_app", "false")
            ]
            .into()
        }));
        assert!(!no_matcher(Frame {
            fields: [
                ("path", "/test.c"),
                ("family", "native"),
                ("in_app", "true")
            ]
            .into()
        }));
    }

    #[test]
    fn package_matching() {
        let bundled_matcher =
            create_matcher("family:native package:/var/**/Frameworks/**                  -app");

        assert!(bundled_matcher(Frame {
            fields: [
                ("package", "/var/containers/MyApp/Frameworks/libsomething"),
                ("family", "native")
            ]
            .into()
        }));
        assert!(!bundled_matcher(Frame {
            fields: [
                ("package", "/var2/containers/MyApp/Frameworks/libsomething"),
                ("family", "native")
            ]
            .into()
        }));
        assert!(!bundled_matcher(Frame {
            fields: [
                ("package", "/var/containers/MyApp/MacOs/MyApp"),
                ("family", "native")
            ]
            .into()
        }));
        assert!(!bundled_matcher(Frame {
            fields: [("package", "/usr/lib/linux-gate.so"), ("family", "native")].into()
        }));

        let macos_matcher =
            create_matcher("family:native package:**/*.app/Contents/**                   +app");

        assert!(macos_matcher(Frame {
            fields: [
                (
                    "package",
                    "/Applications/MyStuff.app/Contents/MacOS/MyStuff"
                ),
                ("family", "native")
            ]
            .into()
        }));

        let linux_matcher =
            create_matcher("family:native package:linux-gate.so                          -app");

        assert!(linux_matcher(Frame {
            fields: [("package", "linux-gate.so"), ("family", "native")].into()
        }));

        let windows_matcher =
            create_matcher("family:native package:?:/Windows/**                          -app");

        assert!(windows_matcher(Frame {
            fields: [
                ("package", "D:\\Windows\\System32\\kernel32.dll"),
                ("family", "native")
            ]
            .into()
        }));

        assert!(windows_matcher(Frame {
            fields: [
                ("package", "d:\\windows\\System32\\kernel32.dll"),
                ("family", "native")
            ]
            .into()
        }));
    }
}
