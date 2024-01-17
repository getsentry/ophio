use pyo3::{prelude::*, types::PyDict};
use rust_ophio::enhancers::{Enhancements as RustEnhancements, ExceptionData, Frame, NoopCache};
use smol_str::SmolStr;

fn exception_data_from_pydict(dict: &PyDict) -> ExceptionData {
    let ty = dict
        .get_item("type")
        .ok()
        .flatten()
        .and_then(|t| t.extract().ok() as Option<&str>)
        .map(SmolStr::new);

    let value = dict
        .get_item("value")
        .ok()
        .flatten()
        .and_then(|t| t.extract().ok() as Option<&str>)
        .map(SmolStr::new);

    let mechanism = dict
        .get_item("mechanism")
        .ok()
        .flatten()
        .and_then(|m| m.get_item("type").ok())
        .and_then(|t| t.extract().ok() as Option<&str>)
        .map(SmolStr::new);

    ExceptionData {
        ty,
        value,
        mechanism,
    }
}

fn get_category(object: &PyDict) -> Option<SmolStr> {
    let data: &PyDict = object.get_item("data").ok()??.downcast().ok()?;
    let category: &str = data.get_item("category").ok()??.extract().ok()?;
    Some(SmolStr::new(category))
}

fn get_family(object: &PyDict, platform: Option<&str>) -> Option<SmolStr> {
    let obj_platform: Option<&str> = object.get_item("platform").ok()??.extract().ok();
    let platform = obj_platform.or(platform)?;

    let family = match platform {
        "objc" | "cocoa" | "swift" | "native" | "c" => "native",
        "javascript" | "node" => "javascript",
        _ => "other",
    };

    Some(SmolStr::new(family))
}

fn get_in_app(object: &PyDict) -> bool {
    let Some(raw) = object.get_item("in_app").ok().flatten() else {
        return false;
    };

    raw.extract().unwrap_or_default()
}

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
fn frame_from_pydict(frame: &PyDict) -> Frame {
    todo!()
}
pub fn apply_modifications_to_py_object(frame: &Frame, dict: &PyDict) {}

#[pyclass]
pub struct Enhancements(RustEnhancements);

#[pymethods]
impl Enhancements {
    #[new]
    fn new(input: &str) -> PyResult<Self> {
        let inner = RustEnhancements::parse(input, NoopCache)?;
        Ok(Self(inner))
    }

    fn apply_modifications_to_frames(
        &self,
        frames: Vec<&PyDict>,
        platform: &str,
        exception_data: &PyDict,
    ) {
    }
}
