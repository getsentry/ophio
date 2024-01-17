use smol_str::SmolStr;

pub type StringField = SmolStr;

#[derive(Debug, Clone, Default)]
pub struct Frame {
    pub category: Option<StringField>,
    pub family: Option<StringField>,
    pub function: Option<StringField>,
    pub in_app: bool,
    pub module: Option<StringField>,
    pub package: Option<StringField>,
    pub path: Option<StringField>,
}

#[derive(Debug, Clone, Copy)]
pub enum FrameField {
    Category,
    Family,
    Function,
    Module,
    Package,
    Path,
}

impl Frame {
    pub fn get_field(&self, field: FrameField) -> Option<&StringField> {
        match field {
            FrameField::Category => self.category.as_ref(),
            FrameField::Family => self.family.as_ref(),
            FrameField::Function => self.function.as_ref(),
            FrameField::Module => self.module.as_ref(),
            FrameField::Package => self.package.as_ref(),
            FrameField::Path => self.path.as_ref(),
        }
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn from_test(raw_frame: &serde_json::Value, platform: &str) -> Self {
        Self {
            category: raw_frame
                .pointer("/data/category")
                .and_then(|s| s.as_str())
                .map(SmolStr::new),
            family: raw_frame
                .get("platform")
                .and_then(|s| s.as_str())
                .or(Some(platform))
                .map(SmolStr::new),
            function: raw_frame
                .get("function")
                .and_then(|s| s.as_str())
                .map(SmolStr::new),
            in_app: raw_frame
                .get("in_app")
                .and_then(|s| s.as_bool())
                .unwrap_or_default(),

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
        }
    }
}
