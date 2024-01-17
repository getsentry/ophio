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

    // TODO:
    pub fn from_py_object() -> Self {
        /*

        // normalize path:
        let mut value = value.replace('\\', "/");

        def create_match_frame(frame_data: dict, platform: Optional[str]) -> dict:
            """Create flat dict of values relevant to matchers"""
            match_frame = dict(
                category=get_path(frame_data, "data", "category"),
                family=get_behavior_family_for_platform(frame_data.get("platform") or platform),
                function=_get_function_name(frame_data, platform),
                in_app=frame_data.get("in_app") or False,
                module=get_path(frame_data, "module"),
                package=frame_data.get("package"),
                path=frame_data.get("abs_path") or frame_data.get("filename"),
            )

            for key in list(match_frame.keys()):
                value = match_frame[key]
                if isinstance(value, (bytes, str)):
                    if key in ("package", "path"):
                        value = match_frame[key] = value.lower()

                    if isinstance(value, str):
                        match_frame[key] = value.encode("utf-8")

            return match_frame
              */
        Self::default()
    }

    // TODO:
    pub fn apply_modifications_to_py_object(&self) {}

    #[cfg(test)]
    pub fn from_test(raw_frame: &serde_json::Value, platform: &str) -> Self {
        let mut frame = Self::default();

        frame.category = raw_frame
            .pointer("/data/category")
            .and_then(|s| s.as_str())
            .map(SmolStr::new);
        frame.family = raw_frame
            .get("platform")
            .and_then(|s| s.as_str())
            .or(Some(platform))
            .map(SmolStr::new);
        frame.function = raw_frame
            .get("function")
            .and_then(|s| s.as_str())
            .map(SmolStr::new);
        frame.in_app = raw_frame
            .get("in_app")
            .and_then(|s| s.as_bool())
            .unwrap_or_default();

        frame.module = raw_frame
            .get("module")
            .and_then(|s| s.as_str())
            .map(SmolStr::new);

        frame.package = raw_frame
            .get("package")
            .and_then(|s| s.as_str())
            .map(|s| SmolStr::new(s.replace('\\', "/").to_lowercase()));

        frame.path = raw_frame
            .get("abs_path")
            .or(raw_frame.get("filename"))
            .and_then(|s| s.as_str())
            .map(|s| SmolStr::new(s.replace('\\', "/").to_lowercase()));

        frame
    }
}
