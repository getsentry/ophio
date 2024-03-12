//! Types for stack frames.

use std::fmt;

use smol_str::SmolStr;

use super::{families::Families, Rule};

pub type StringField = SmolStr;

/// Represents a stack frame for the purposes of grouping rules.
#[derive(Debug, Clone, Default)]
pub struct Frame {
    /// The frame's category (e.g. `"telemetry"`, `"ui"`, &c.)
    pub category: Option<StringField>,
    /// The frame's family (`"native"`, `"javascript"`, or `"other"`), represented
    /// compactly as a bit field.
    pub family: Families,
    /// The frame's function name.
    pub function: Option<StringField>,
    /// The frame's module name.
    pub module: Option<StringField>,
    /// The frame's package name.
    pub package: Option<StringField>,
    /// The frame's path.
    pub path: Option<StringField>,

    /// The frame's in_app flag.
    ///
    /// This denotes whether we consider this frame to have
    /// originated from within the user's code (`true`) or from
    /// system libraries, frameworks, &c. (`false`).
    pub in_app: Option<bool>,
    /// The [`Rule`] that last modified this frame's `in_app` field.
    ///
    /// This is used for keeping track of grouping contribution information,
    /// see [`update_frame_components_contributions`](super::Enhancements::assemble_stacktrace_component).
    pub in_app_last_changed: Option<Rule>,
}

/// The name of a string-valued field in a frame.
#[derive(Debug, Clone, Copy)]
pub enum FrameField {
    Category,
    Function,
    Module,
    Package,
    Path,
    // NOTE: This is only used to have something to `Display` in the `Noop` matcher.
    App,
}

impl fmt::Display for FrameField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameField::Category => write!(f, "category"),
            FrameField::Function => write!(f, "function"),
            FrameField::Module => write!(f, "module"),
            FrameField::Package => write!(f, "package"),
            FrameField::Path => write!(f, "path"),
            FrameField::App => write!(f, "app"),
        }
    }
}

impl Frame {
    /// Gets the value of `field` from `self`.
    pub fn get_field(&self, field: FrameField) -> Option<&StringField> {
        match field {
            FrameField::Category => self.category.as_ref(),
            FrameField::Function => self.function.as_ref(),
            FrameField::Module => self.module.as_ref(),
            FrameField::Package => self.package.as_ref(),
            FrameField::Path => self.path.as_ref(),
            // NOTE: we never *access* the field via `get_field`.
            FrameField::App => unreachable!(),
        }
    }

    /// Convenience constructor for use within tests.
    #[cfg(any(test, feature = "testing"))]
    pub fn from_test(raw_frame: &serde_json::Value, platform: &str) -> Self {
        Self {
            category: raw_frame
                .pointer("/data/category")
                .and_then(|s| s.as_str())
                .map(SmolStr::new),
            family: Families::new(
                raw_frame
                    .get("platform")
                    .and_then(|s| s.as_str())
                    .unwrap_or(platform),
            ),
            function: raw_frame
                .get("function")
                .and_then(|s| s.as_str())
                .map(SmolStr::new),
            in_app: raw_frame.get("in_app").and_then(|s| s.as_bool()),

            module: raw_frame
                .get("module")
                .and_then(|s| s.as_str())
                .map(SmolStr::new),

            package: raw_frame
                .get("package")
                .and_then(|s| s.as_str())
                .map(|s| SmolStr::new(s.replace('\\', "/").to_lowercase())),

            path: raw_frame
                .get("abs_path")
                .or(raw_frame.get("filename"))
                .and_then(|s| s.as_str())
                .map(|s| SmolStr::new(s.replace('\\', "/").to_lowercase())),
            in_app_last_changed: None,
        }
    }
}
