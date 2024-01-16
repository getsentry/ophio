use std::collections::HashSet;
use std::sync::Arc;

use globset::GlobBuilder;
use regex::bytes::{Regex, RegexBuilder};
use smol_str::SmolStr;

use super::frame::{Frame, FrameField};
use super::ExceptionData;

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

#[derive(Clone)]
pub enum Matcher {
    Frame(Arc<dyn FrameMatcher>),
    Exception(Arc<dyn ExceptionMatcher>),
}

// TODO: take `caller/e` as argument
pub fn get_matcher(
    negated: bool,
    matcher_type: &str,
    argument: &str,
    caller: bool,
    callee: bool,
) -> anyhow::Result<Matcher> {
    let matcher = match matcher_type {
        // Field matchers
        "stack.module" | "module" => Matcher::Frame(create_frame_matcher(
            negated,
            caller,
            callee,
            FrameFieldMatch::new(FrameField::Module, argument)?,
        )),
        "stack.function" | "function" => Matcher::Frame(create_frame_matcher(
            negated,
            caller,
            callee,
            FrameFieldMatch::new(FrameField::Function, argument)?,
        )),
        "category" => Matcher::Frame(create_frame_matcher(
            negated,
            caller,
            callee,
            FrameFieldMatch::new(FrameField::Category, argument)?,
        )),

        // Path matchers
        "stack.abs_path" | "path" => Matcher::Frame(create_frame_matcher(
            negated,
            caller,
            callee,
            PathLikeMatch::new(FrameField::Path, argument)?,
        )),
        "stack.package" | "package" => Matcher::Frame(create_frame_matcher(
            negated,
            caller,
            callee,
            PathLikeMatch::new(FrameField::Package, argument)?,
        )),

        // Family matcher
        "family" => Matcher::Frame(create_frame_matcher(
            negated,
            caller,
            callee,
            FamilyMatch::new(argument),
        )),

        // InApp matcher
        "app" => Matcher::Frame(create_frame_matcher(
            negated,
            caller,
            callee,
            InAppMatch::new(argument)?,
        )),

        // Exception matchers
        "error.type" | "type" => Matcher::Exception(create_exception_matcher(
            negated,
            ExceptionTypeMatch::new(argument)?,
        )),
        "error.value" | "value" => Matcher::Exception(create_exception_matcher(
            negated,
            ExceptionValueMatch::new(argument)?,
        )),
        "error.mechanism" | "mechanism" => Matcher::Exception(create_exception_matcher(
            negated,
            ExceptionMechanismMatch::new(argument)?,
        )),

        matcher_type => anyhow::bail!("Unknown matcher `{matcher_type}`"),
    };

    Ok(matcher)
}

pub trait FrameMatcher {
    fn matches_frame(&self, frames: &[Frame], idx: usize) -> bool;
}

trait SimpleFieldMatcher {
    fn field(&self) -> FrameField;
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

pub fn create_frame_matcher<M: FrameMatcher + 'static>(
    negated: bool,
    caller: bool,
    callee: bool,
    matcher: M,
) -> Arc<dyn FrameMatcher> {
    if caller {
        Arc::new(CallerMatch(NegationWrapper {
            negated,
            inner: matcher,
        }))
    } else if callee {
        Arc::new(CalleeMatch(NegationWrapper {
            negated,
            inner: matcher,
        }))
    } else {
        Arc::new(NegationWrapper {
            negated,
            inner: matcher,
        })
    }
}

#[derive(Debug, Clone)]
pub struct FrameFieldMatch {
    field: FrameField, // function, module, category
    pattern: Regex,
}

impl FrameFieldMatch {
    pub fn new(field: FrameField, pattern: &str) -> anyhow::Result<Self> {
        let pattern = translate_pattern(pattern, false)?;

        Ok(Self { field, pattern })
    }
}

impl SimpleFieldMatcher for FrameFieldMatch {
    fn field(&self) -> FrameField {
        self.field
    }
    fn matches_value(&self, value: &str) -> bool {
        self.pattern.is_match(value.as_bytes())
    }
}

#[derive(Debug, Clone)]
pub struct PathLikeMatch {
    field: FrameField, // package, path
    pattern: Regex,
}

impl PathLikeMatch {
    pub fn new(field: FrameField, pattern: &str) -> anyhow::Result<Self> {
        let pattern = translate_pattern(pattern, true)?;

        Ok(Self { field, pattern })
    }
}

impl SimpleFieldMatcher for PathLikeMatch {
    fn field(&self) -> FrameField {
        self.field
    }

    fn matches_value(&self, value: &str) -> bool {
        if self.pattern.is_match(value.as_bytes()) {
            return true;
        }
        if !value.starts_with('/') {
            // TODO: avoid
            let value = format!("/{value}");
            return self.pattern.is_match(value.as_bytes());
        }
        false
    }
}

#[derive(Debug, Clone)]
<<<<<<< HEAD
struct FamilyMatch {
    // NOTE: This is a `Vec` because we typically only have a single item.
    // NOTE: we optimize for `"all"` by just storing an empty `Vec` and checking for that
    families: Vec<SmolStr>,
}

impl FamilyMatch {
    fn new(families: &str) -> Self {
        let mut families: Vec<_> = families.split(',').map(SmolStr::from).collect();
        if families.contains(&SmolStr::new("all")) {
            families = vec![];
        }
=======
pub struct FamilyMatch {
    families: HashSet<SmolStr>,
}

impl FamilyMatch {
    pub fn new(families: &str) -> Self {
        let families = families.split(',').map(SmolStr::from).collect();
>>>>>>> d5edbed (Parse directly with nom)

        Self { families }
    }
}

impl SimpleFieldMatcher for FamilyMatch {
    fn field(&self) -> FrameField {
        FrameField::Family
    }

    fn matches_value(&self, value: &str) -> bool {
        self.families.is_empty() || self.families.iter().any(|el| el == value)
    }
}

#[derive(Debug, Clone)]
pub struct InAppMatch {
    expected: bool,
}

impl InAppMatch {
    pub fn new(expected: &str) -> anyhow::Result<Self> {
        match expected {
            "1" | "true" | "yes" => Ok(Self { expected: true }),
            "0" | "false" | "no" => Ok(Self { expected: false }),
            _ => Err(anyhow::anyhow!("Invalid value for `app`: `{expected}`")),
        }
    }
}

impl FrameMatcher for InAppMatch {
    fn matches_frame(&self, frames: &[Frame], idx: usize) -> bool {
        let Some(frame) = frames.get(idx) else {
            return false;
        };

        frame.in_app == self.expected
    }
}

#[derive(Debug, Clone)]
pub struct CallerMatch<M>(M);

impl<M: FrameMatcher> FrameMatcher for CallerMatch<M> {
    fn matches_frame(&self, frames: &[Frame], idx: usize) -> bool {
        idx > 0 && self.0.matches_frame(frames, idx - 1)
    }
}

#[derive(Debug, Clone)]
pub struct CalleeMatch<M>(M);

impl<M: FrameMatcher> FrameMatcher for CalleeMatch<M> {
    fn matches_frame(&self, frames: &[Frame], idx: usize) -> bool {
        !frames.is_empty() && idx < frames.len() - 1 && self.0.matches_frame(frames, idx + 1)
    }
}

pub trait ExceptionMatcher {
    fn matches_exception(&self, exception_data: &ExceptionData) -> bool;
}

pub struct ExceptionTypeMatch {
    pattern: Regex,
}

impl ExceptionTypeMatch {
    pub fn new(pattern: &str) -> anyhow::Result<Self> {
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

pub struct ExceptionValueMatch {
    pattern: Regex,
}

impl ExceptionValueMatch {
    pub fn new(pattern: &str) -> anyhow::Result<Self> {
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

pub struct ExceptionMechanismMatch {
    pattern: Regex,
}

impl ExceptionMechanismMatch {
    pub fn new(pattern: &str) -> anyhow::Result<Self> {
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

pub fn create_exception_matcher<M: ExceptionMatcher + 'static>(
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
    use serde_json::json;

    use crate::enhancers::grammar::parse_enhancers;

    use super::*;

    fn create_matcher(rules: &str) -> impl Fn(Frame) -> bool {
        let rules = parse_enhancers(rules).unwrap();
        let rule = rules.rules.into_iter().next().unwrap();
        let matchers = rule.frame_matchers;

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

        assert!(matcher(Frame::from_test(
            &json!({"abs_path": "http://example.com/foo/test.js", "filename": "/foo/test.js"}),
            "javascript"
        )));

        assert!(!matcher(Frame::from_test(
            &json!({"abs_path": "http://example.com/foo/bar.js", "filename": "/foo/bar.js"}),
            "javascript"
        )));

        assert!(matcher(Frame::from_test(
            &json!({"abs_path": "http://example.com/foo/test.js"}),
            "javascript"
        )));

        assert!(!matcher(Frame::from_test(
            &json!({"filename": "/foo/bar.js"}),
            "javascript"
        )));

        assert!(matcher(Frame::from_test(
            &json!({"abs_path": "http://example.com/foo/TEST.js"}),
            "javascript"
        )));

        assert!(!matcher(Frame::from_test(
            &json!({"abs_path": "http://example.com/foo/bar.js"}),
            "javascript"
        )));
    }

    #[test]
    fn family_matching() {
        let js_matcher = create_matcher("family:javascript path:**/test.js              +app");
        let native_matcher = create_matcher("family:native function:std::*                  -app");

        assert!(js_matcher(Frame::from_test(
            &json!({"abs_path": "http://example.com/foo/TEST.js"}),
            "javascript"
        )));
        assert!(!js_matcher(Frame::from_test(
            &json!({"abs_path": "http://example.com/foo/TEST.js"}),
            "native"
        )));

        assert!(!native_matcher(Frame::from_test(
            &json!({"abs_path": "http://example.com/foo/TEST.js", "function": "std::whatever"}),
            "javascript"
        )));
        assert!(native_matcher(Frame::from_test(
            &json!({"function": "std::whatever"}),
            "native"
        )));
    }

    #[test]
    fn app_matching() {
        let yes_matcher = create_matcher("family:javascript path:**/test.js app:yes       +app");
        let no_matcher = create_matcher("family:native path:**/test.c app:no            -group");

        assert!(yes_matcher(Frame::from_test(
            &json!({"abs_path": "http://example.com/foo/TEST.js", "in_app": true}),
            "javascript"
        )));
        assert!(!yes_matcher(Frame::from_test(
            &json!({"abs_path": "http://example.com/foo/TEST.js", "in_app": false}),
            "javascript"
        )));
        assert!(no_matcher(Frame::from_test(
            &json!({"abs_path": "/test.c", "in_app": false}),
            "native"
        )));
        assert!(!no_matcher(Frame::from_test(
            &json!({"abs_path": "/test.c", "in_app":true}),
            "native"
        )));
    }

    #[test]
    fn package_matching() {
        let bundled_matcher =
            create_matcher("family:native package:/var/**/Frameworks/**                  -app");

        assert!(bundled_matcher(Frame::from_test(
            &json!({"package": "/var/containers/MyApp/Frameworks/libsomething"}),
            "native"
        )));
        assert!(!bundled_matcher(Frame::from_test(
            &json!({"package": "/var2/containers/MyApp/Frameworks/libsomething"}),
            "native"
        )));
        assert!(!bundled_matcher(Frame::from_test(
            &json!({"package": "/var/containers/MyApp/MacOs/MyApp"}),
            "native"
        )));
        assert!(!bundled_matcher(Frame::from_test(
            &json!({"package": "/usr/lib/linux-gate.so"}),
            "native"
        )));

        let macos_matcher =
            create_matcher("family:native package:**/*.app/Contents/**                   +app");

        assert!(macos_matcher(Frame::from_test(
            &json!({"package": "/Applications/MyStuff.app/Contents/MacOS/MyStuff"}),
            "native"
        )));

        let linux_matcher =
            create_matcher("family:native package:linux-gate.so                          -app");

        assert!(linux_matcher(Frame::from_test(
            &json!({"package": "linux-gate.so"}),
            "native"
        )));

        let windows_matcher =
            create_matcher("family:native package:?:/Windows/**                          -app");

        assert!(windows_matcher(Frame::from_test(
            &json!({"package": "D:\\Windows\\System32\\kernel32.dll"}),
            "native"
        )));

        assert!(windows_matcher(Frame::from_test(
            &json!({"package": "d:\\windows\\System32\\kernel32.dll"}),
            "native"
        )));
    }
}
