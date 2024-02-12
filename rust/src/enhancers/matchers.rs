//! Matchers represent certain predicates on frames or exceptions.
//!
//! Every [`Matcher`] is either a [`FrameMatcher`] or an [`ExceptionMatcher`]. A [`FrameMatcher`] checks a stack frame
//! against a given condition—typically, whether a certain field conforms to a regex pattern. [`ExceptionMatchers`](ExceptionMatcher)
//! do the same for exceptions.
//!
//! See <https://docs.sentry.io/product/data-management-settings/event-grouping/stack-trace-rules/#matchers> for an explanation of how
//! the various matchers work.

use std::fmt;
use std::sync::Arc;

use regex::bytes::Regex;
use smol_str::SmolStr;

use super::families::Families;
use super::frame::{Frame, FrameField};
use super::{ExceptionData, RegexCache};

/// Enum that wraps a frame or exception matcher.
///
/// This exists mostly to allow parsing both frame and exception matchers uniformly.
#[derive(Debug, Clone)]
pub(crate) enum Matcher {
    Frame(FrameMatcher),
    Exception(ExceptionMatcher),
}

impl Matcher {
    /// Creates an instance of [`Self::Frame`].
    fn new_frame(
        negated: bool,
        frame_offset: FrameOffset,
        inner: FrameMatcherInner,
        raw_pattern: &str,
    ) -> Self {
        Self::Frame(FrameMatcher {
            negated,
            frame_offset,
            inner,
            raw_pattern: SmolStr::new(raw_pattern),
        })
    }

    /// Creates a matcher from string arguments.
    ///
    /// # Parameters
    /// * `negated`: Whether the matcher should be negated.
    /// * `matcher_type`: The matcher's type, e.g. `module` or `mechanism`.
    /// * `raw_pattern`: The raw pattern values are matched against. This argument's format depends
    ///   on the matcher type: for `app`, it is a pseudo-boolean; for `family`, a comma-separated list
    ///   of families; for all others, a glob pattern.
    /// * frame_offset: Determines whether this matcher should match a frame by checking the frame itself
    ///   or one of its adjacent frames. This only applies to frame matchers, not exception matchers.
    /// * `regex_cache`: A cache for regexes.
    pub(crate) fn new(
        negated: bool,
        matcher_type: &str,
        raw_pattern: &str,
        frame_offset: FrameOffset,
        regex_cache: &mut RegexCache,
    ) -> anyhow::Result<Self> {
        match matcher_type {
            // Field matchers
            "stack.module" | "module" => Ok(Self::new_frame(
                negated,
                frame_offset,
                FrameMatcherInner::new_field(FrameField::Module, false, raw_pattern, regex_cache)?,
                raw_pattern,
            )),
            "stack.function" | "function" => Ok(Self::new_frame(
                negated,
                frame_offset,
                FrameMatcherInner::new_field(
                    FrameField::Function,
                    false,
                    raw_pattern,
                    regex_cache,
                )?,
                raw_pattern,
            )),
            "category" => Ok(Self::new_frame(
                negated,
                frame_offset,
                FrameMatcherInner::new_field(
                    FrameField::Category,
                    false,
                    raw_pattern,
                    regex_cache,
                )?,
                raw_pattern,
            )),

            // Path matchers
            "stack.abs_path" | "path" => Ok(Self::new_frame(
                negated,
                frame_offset,
                FrameMatcherInner::new_field(FrameField::Path, true, raw_pattern, regex_cache)?,
                raw_pattern,
            )),
            "stack.package" | "package" => Ok(Self::new_frame(
                negated,
                frame_offset,
                FrameMatcherInner::new_field(FrameField::Package, true, raw_pattern, regex_cache)?,
                raw_pattern,
            )),

            // Family matcher
            "family" => Ok(Self::new_frame(
                negated,
                frame_offset,
                FrameMatcherInner::new_family(raw_pattern),
                raw_pattern,
            )),

            // InApp matcher
            "app" => Ok(Self::new_frame(
                negated,
                frame_offset,
                FrameMatcherInner::new_in_app(raw_pattern)?,
                raw_pattern,
            )),

            // Exception matchers
            "error.type" | "type" => Ok(Self::Exception(ExceptionMatcher::new_type(
                negated,
                raw_pattern,
                regex_cache,
            )?)),

            "error.value" | "value" => Ok(Self::Exception(ExceptionMatcher::new_value(
                negated,
                raw_pattern,
                regex_cache,
            )?)),

            "error.mechanism" | "mechanism" => Ok(Self::Exception(
                ExceptionMatcher::new_mechanism(negated, raw_pattern, regex_cache)?,
            )),

            matcher_type => anyhow::bail!("Unknown matcher `{matcher_type}`"),
        }
    }
}

/// Denotes whether a frame matcher applies to the current frame or one of the adjacent frames.
#[derive(Debug, Clone, Copy)]
pub(crate) enum FrameOffset {
    /// The caller frame, i.e., the one before the current frame.
    Caller,
    /// The callee frame, i.e., the one after the current frame.
    Callee,
    /// The current frame.
    None,
}

/// A component for telling whether a frame matches a certain predicate.
///
/// This wraps a [`FrameMatcherInner`], which does the actual matching, with some extra logic.
#[derive(Debug, Clone)]
pub struct FrameMatcher {
    /// If this is true, a frame passes the matcher if it *doesn't* pass the inner matcher.
    negated: bool,
    /// The frame this matcher applies to (the current one, the caller, or the callee).
    frame_offset: FrameOffset,
    /// The inner matcher that actually contains the matching logic.
    inner: FrameMatcherInner,
    /// The string pattern this matcher was constructed from. This is used for the `Display` impl.
    raw_pattern: SmolStr,
}

impl FrameMatcher {
    /// Tests whether the `i`th frame in `frames` matches.
    ///
    /// Fundamentally this calles `self.inner.matches_frame`. If `self.negated` is true,
    /// that method's result will be flipped. `self.frame_offset` controls whether
    /// `inner.matches_frame` is called on `frames[i]` or one of the adjacent frames.
    pub fn matches_frame(&self, frames: &[Frame], idx: usize) -> bool {
        let idx = match self.frame_offset {
            FrameOffset::Caller => idx.checked_sub(1),
            FrameOffset::Callee => idx.checked_add(1),
            FrameOffset::None => Some(idx),
        };

        let Some(idx) = idx else {
            return false;
        };

        let Some(frame) = frames.get(idx) else {
            return false;
        };

        self.negated ^ self.inner.matches_frame(frame)
    }
}

impl fmt::Display for FrameMatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let FrameMatcher {
            negated,
            frame_offset,
            inner,
            raw_pattern,
        } = self;

        match frame_offset {
            FrameOffset::Caller => write!(f, "[")?,
            FrameOffset::Callee => write!(f, "| [")?,
            FrameOffset::None => {}
        }

        if *negated {
            write!(f, "!")?;
        }

        write!(f, "{inner}:{raw_pattern}")?;

        match frame_offset {
            FrameOffset::Caller => write!(f, "] |")?,
            FrameOffset::Callee => write!(f, "]")?,
            FrameOffset::None => {}
        }

        Ok(())
    }
}

/// A component for telling whether a frame matches a certain predicate.
///
/// This type is not used directly, but rather wrapped in a [`FrameMatcher`].
#[derive(Debug, Clone)]
enum FrameMatcherInner {
    /// Checks whether a particular field of a frame conforms to a pattern.
    Field {
        /// The field to check.
        field: FrameField,
        /// Whether the field contains a "path-like" value.
        ///
        /// If this is true, backslashes will be normalized
        /// to slashes in both the pattern and the value, among other things.
        path_like: bool,
        /// The regex pattern to check the frame field against.
        pattern: Arc<Regex>,
    },
    /// Checks whether a frame's `family` field is one of the allowed families.
    Family { families: Families },
    /// Checks whether a frame's in_app field is equal to an expected value.
    InApp { expected: bool },
    /// A matcher that will never match.
    Noop {
        /// The field to check.
        field: FrameField,
    },
}

impl FrameMatcherInner {
    /// Creates a matcher that checks a frame field.
    fn new_field(
        field: FrameField,
        path_like: bool,
        pattern: &str,
        regex_cache: &mut RegexCache,
    ) -> anyhow::Result<Self> {
        let Ok(pattern) = regex_cache.get_or_try_insert(pattern, path_like) else {
            // TODO: we should be returning real errors in a `strict` parsing mode
            return Ok(Self::Noop { field });
        };

        Ok(Self::Field {
            field,
            path_like,
            pattern,
        })
    }

    /// Creates a matcher that checks a frame's family.
    fn new_family(families: &str) -> Self {
        Self::Family {
            families: Families::new(families),
        }
    }

    /// Creates a matcher that checks a frame's `in_app` field.
    fn new_in_app(expected: &str) -> anyhow::Result<Self> {
        match expected {
            "1" | "true" | "yes" => Ok(Self::InApp { expected: true }),
            "0" | "false" | "no" => Ok(Self::InApp { expected: false }),
            _ => Ok(Self::Noop {
                field: FrameField::App,
            }),
            // TODO: we should be returning real errors in a `strict` parsing mode
            // _ => Err(anyhow::anyhow!("Invalid value for `app`: `{expected}`")),
        }
    }

    /// Checks whether a frame matches.
    fn matches_frame(&self, frame: &Frame) -> bool {
        match self {
            FrameMatcherInner::Field {
                field,
                path_like,
                pattern,
            } => {
                let Some(value) = frame.get_field(*field) else {
                    return false;
                };

                if pattern.is_match(value.as_bytes()) {
                    return true;
                }

                if *path_like && !value.starts_with('/') {
                    // TODO: avoid
                    let value = format!("/{value}");
                    return pattern.is_match(value.as_bytes());
                }
                false
            }
            FrameMatcherInner::Family { families } => families.matches(frame.family),
            FrameMatcherInner::InApp { expected } => frame.in_app.unwrap_or_default() == *expected,
            FrameMatcherInner::Noop { .. } => false,
        }
    }
}

impl fmt::Display for FrameMatcherInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameMatcherInner::Field { field, .. } | FrameMatcherInner::Noop { field } => {
                write!(f, "{field}")
            }
            FrameMatcherInner::Family { .. } => write!(f, "family"),
            FrameMatcherInner::InApp { .. } => write!(f, "app"),
        }
    }
}

/// Which field an exception matcher checks.
#[derive(Debug, Clone, Copy)]
enum ExceptionMatcherType {
    /// Checks the `type` field.
    Type,
    /// Checks the `value` field.
    Value,
    /// Checks the `mechanism.type` field.
    Mechanism,
}

impl fmt::Display for ExceptionMatcherType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExceptionMatcherType::Type => write!(f, "type"),
            ExceptionMatcherType::Value => write!(f, "value"),
            ExceptionMatcherType::Mechanism => write!(f, "mechanism"),
        }
    }
}

/// A component for telling whether an exception matches a certain predicate.
#[derive(Debug, Clone)]
pub struct ExceptionMatcher {
    /// If this is true, an exception passes the matcher if
    /// its relevant field *doesn't* fit the pattern.
    negated: bool,
    /// The regex pattern to check the exception field against.
    pattern: Arc<Regex>,
    /// The field to check.
    ty: ExceptionMatcherType,
    /// The string pattern this matcher was constructed from. This is used for the `Display` impl.
    raw_pattern: SmolStr,
}

impl ExceptionMatcher {
    /// Creates a matcher that checks an exception's `type` field.
    fn new_type(
        negated: bool,
        raw_pattern: &str,
        regex_cache: &mut RegexCache,
    ) -> anyhow::Result<Self> {
        let pattern = regex_cache.get_or_try_insert(raw_pattern, false)?;
        Ok(Self {
            negated,
            pattern,
            ty: ExceptionMatcherType::Type,
            raw_pattern: SmolStr::new(raw_pattern),
        })
    }

    /// Creates a matcher that checks an exception's `value` field.
    fn new_value(
        negated: bool,
        raw_pattern: &str,
        regex_cache: &mut RegexCache,
    ) -> anyhow::Result<Self> {
        let pattern = regex_cache.get_or_try_insert(raw_pattern, false)?;
        Ok(Self {
            negated,
            pattern,
            ty: ExceptionMatcherType::Value,
            raw_pattern: SmolStr::new(raw_pattern),
        })
    }

    /// Creates a matcher that checks an exception's `mechanism` field.
    fn new_mechanism(
        negated: bool,
        raw_pattern: &str,
        regex_cache: &mut RegexCache,
    ) -> anyhow::Result<Self> {
        let pattern = regex_cache.get_or_try_insert(raw_pattern, false)?;
        Ok(Self {
            negated,
            pattern,
            ty: ExceptionMatcherType::Mechanism,
            raw_pattern: SmolStr::new(raw_pattern),
        })
    }

    /// Checks whether an exception matches.
    pub fn matches_exception(&self, exception_data: &ExceptionData) -> bool {
        let value = match self.ty {
            ExceptionMatcherType::Type => &exception_data.ty,
            ExceptionMatcherType::Value => &exception_data.value,
            ExceptionMatcherType::Mechanism => &exception_data.mechanism,
        };

        let value = value.as_deref().unwrap_or("<unknown>").as_bytes();
        self.negated ^ self.pattern.is_match(value)
    }
}

impl fmt::Display for ExceptionMatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ExceptionMatcher {
            negated,
            raw_pattern,
            ty,
            ..
        } = self;

        if *negated {
            write!(f, "!")?;
        }

        write!(f, "{ty}:{raw_pattern}")
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::enhancers::Enhancements;

    use super::*;

    fn create_matcher(input: &str) -> impl Fn(Frame) -> bool {
        let enhancements = Enhancements::parse(input, &mut Default::default()).unwrap();
        let rule = enhancements.all_rules.into_iter().next().unwrap();

        move |frame: Frame| {
            let frames = &[frame];
            rule.matches_frame(frames, 0)
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

    #[test]
    fn test_dtor() {
        let matcher = create_matcher(r#"family:native function:"*::\\{dtor\\}" category=dtor"#);
        assert!(matcher(Frame::from_test(
            &json!({"function": "abccore::classref::InterfaceRef<T>::{dtor}"}),
            "native"
        )));
    }
}
