use std::fmt;
use std::sync::Arc;

use globset::GlobBuilder;
use regex::bytes::{Regex, RegexBuilder};
use smol_str::SmolStr;

use super::frame::{Frame, FrameField};
use super::{Cache, ExceptionData};

/// Enum that wraps a frame or exception matcher.
///
/// This exists mostly to allow parsing both frame and exception matchers uniformly.
#[derive(Debug, Clone)]
pub(crate) enum Matcher {
    Frame(FrameMatcher),
    Exception(ExceptionMatcher),
}

impl Matcher {
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

    pub(crate) fn new(
        negated: bool,
        matcher_type: &str,
        argument: &str,
        frame_offset: FrameOffset,
        cache: &mut Cache,
    ) -> anyhow::Result<Self> {
        match matcher_type {
            // Field matchers
            "stack.module" | "module" => Ok(Self::new_frame(
                negated,
                frame_offset,
                FrameMatcherInner::new_field(FrameField::Module, false, argument, cache)?,
                argument,
            )),
            "stack.function" | "function" => Ok(Self::new_frame(
                negated,
                frame_offset,
                FrameMatcherInner::new_field(FrameField::Function, false, argument, cache)?,
                argument,
            )),
            "category" => Ok(Self::new_frame(
                negated,
                frame_offset,
                FrameMatcherInner::new_field(FrameField::Category, false, argument, cache)?,
                argument,
            )),

            // Path matchers
            "stack.abs_path" | "path" => Ok(Self::new_frame(
                negated,
                frame_offset,
                FrameMatcherInner::new_field(FrameField::Path, true, argument, cache)?,
                argument,
            )),
            "stack.package" | "package" => Ok(Self::new_frame(
                negated,
                frame_offset,
                FrameMatcherInner::new_field(FrameField::Package, true, argument, cache)?,
                argument,
            )),

            // Family matcher
            "family" => Ok(Self::new_frame(
                negated,
                frame_offset,
                FrameMatcherInner::new_family(argument),
                argument,
            )),

            // InApp matcher
            "app" => Ok(Self::new_frame(
                negated,
                frame_offset,
                FrameMatcherInner::new_in_app(argument)?,
                argument,
            )),

            // Exception matchers
            "error.type" | "type" => Ok(Self::Exception(ExceptionMatcher::new_type(
                negated, argument, cache,
            )?)),

            "error.value" | "value" => Ok(Self::Exception(ExceptionMatcher::new_value(
                negated, argument, cache,
            )?)),

            "error.mechanism" | "mechanism" => Ok(Self::Exception(
                ExceptionMatcher::new_mechanism(negated, argument, cache)?,
            )),

            matcher_type => anyhow::bail!("Unknown matcher `{matcher_type}`"),
        }
    }
}

/// Denotes whether a frame matcher applies to the current frame or one of the adjacent frames.
#[derive(Debug, Clone, Copy)]
pub(crate) enum FrameOffset {
    Caller,
    Callee,
    None,
}

#[derive(Debug, Clone)]
pub struct FrameMatcher {
    negated: bool,
    frame_offset: FrameOffset,
    inner: FrameMatcherInner,
    raw_pattern: SmolStr,
}

impl FrameMatcher {
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

#[derive(Debug, Clone)]
enum FrameMatcherInner {
    /// Checks whether a particular field of a frame conforms to a pattern.
    Field {
        field: FrameField,
        path_like: bool,
        pattern: Arc<Regex>,
    },
    /// Checks whether a frame's family is one of the allowed families.
    Family { native: bool, javascript: bool },
    /// Checks whether a frame's in_app field is equal to an expected value.
    InApp { expected: bool },
}

impl FrameMatcherInner {
    fn new_field(
        field: FrameField,
        path_like: bool,
        pattern: &str,
        cache: &mut Cache,
    ) -> anyhow::Result<Self> {
        let pattern = cache.get_or_try_insert_regex(pattern, path_like, translate_pattern)?;
        Ok(Self::Field {
            field,
            path_like,
            pattern,
        })
    }

    fn new_family(families: &str) -> Self {
        let (mut native, mut javascript) = (false, false);

        for f in families.split(',') {
            match f {
                "native" => native = true,
                "javascript" => javascript = true,
                "all" => {
                    native = true;
                    javascript = true;
                    break;
                }
                _ => continue,
            }
        }

        Self::Family { native, javascript }
    }

    fn new_in_app(expected: &str) -> anyhow::Result<Self> {
        match expected {
            "1" | "true" | "yes" => Ok(Self::InApp { expected: true }),
            "0" | "false" | "no" => Ok(Self::InApp { expected: false }),
            _ => Err(anyhow::anyhow!("Invalid value for `app`: `{expected}`")),
        }
    }

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
            FrameMatcherInner::Family { native, javascript } => {
                let Some(value) = frame.get_field(FrameField::Family) else {
                    return false;
                };

                match value.as_ref() {
                    "native" => *native,
                    "javascript" => *javascript,
                    _ => false,
                }
            }
            FrameMatcherInner::InApp { expected } => frame.in_app.unwrap_or_default() == *expected,
        }
    }
}

impl fmt::Display for FrameMatcherInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameMatcherInner::Field { field, .. } => write!(f, "{field}"),
            FrameMatcherInner::Family { .. } => write!(f, "family"),
            FrameMatcherInner::InApp { .. } => write!(f, "app"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ExceptionMatcherType {
    Type,
    Value,
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

#[derive(Debug, Clone)]
pub struct ExceptionMatcher {
    negated: bool,
    pattern: Arc<Regex>,
    ty: ExceptionMatcherType,
    raw_pattern: SmolStr,
}

impl ExceptionMatcher {
    fn new_type(negated: bool, raw_pattern: &str, cache: &mut Cache) -> anyhow::Result<Self> {
        let pattern = cache.get_or_try_insert_regex(raw_pattern, false, translate_pattern)?;
        Ok(Self {
            negated,
            pattern,
            ty: ExceptionMatcherType::Type,
            raw_pattern: SmolStr::new(raw_pattern),
        })
    }

    fn new_value(negated: bool, raw_pattern: &str, cache: &mut Cache) -> anyhow::Result<Self> {
        let pattern = cache.get_or_try_insert_regex(raw_pattern, false, translate_pattern)?;
        Ok(Self {
            negated,
            pattern,
            ty: ExceptionMatcherType::Value,
            raw_pattern: SmolStr::new(raw_pattern),
        })
    }

    fn new_mechanism(negated: bool, raw_pattern: &str, cache: &mut Cache) -> anyhow::Result<Self> {
        let pattern = cache.get_or_try_insert_regex(raw_pattern, false, translate_pattern)?;
        Ok(Self {
            negated,
            pattern,
            ty: ExceptionMatcherType::Mechanism,
            raw_pattern: SmolStr::new(raw_pattern),
        })
    }

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
