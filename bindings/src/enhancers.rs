use pyo3::prelude::*;
use pyo3::types::{PyDict, PyIterator};
use rust_ophio::enhancers;

#[derive(FromPyObject)]
#[pyo3(from_item_all)]
pub struct Frame {
    category: OptStr,
    family: OptStr,
    function: OptStr,
    module: OptStr,
    package: OptStr,
    path: OptStr,
    in_app: bool,
}

struct OptStr(Option<enhancers::StringField>);

impl FromPyObject<'_> for OptStr {
    fn extract(ob: &PyAny) -> PyResult<Self> {
        if ob.is_none() {
            return Ok(Self(None));
        }
        let s: &[u8] = ob.extract()?;
        let s = std::str::from_utf8(s)?;
        Ok(Self(Some(enhancers::StringField::new(s))))
    }
}

#[derive(FromPyObject)]
#[pyo3(from_item_all)]
pub struct ExceptionData {
    ty: OptStr,
    value: OptStr,
    mechanism: OptStr,
}

#[pyclass]
pub struct Cache(enhancers::Cache);

#[pymethods]
impl Cache {
    #[new]
    fn new(size: usize) -> PyResult<Self> {
        Ok(Self(enhancers::Cache::new(size)))
    }
}

#[pyclass]
pub struct Enhancements(enhancers::Enhancements);

#[pymethods]
impl Enhancements {
    #[new]
    fn new(input: &str, cache: &mut Cache) -> PyResult<Self> {
        let inner = enhancers::Enhancements::parse(input, &mut cache.0)?;
        Ok(Self(inner))
    }

    fn apply_modifications_to_frames(
        &self,
        py: Python,
        frames: &PyIterator,
        exception_data: ExceptionData,
    ) -> PyResult<Vec<PyObject>> {
        let mut frames: Vec<_> = frames
            .map(|frame| {
                let frame: Frame = frame?.extract()?;
                let frame = enhancers::Frame {
                    category: frame.category.0,
                    family: frame.family.0,
                    function: frame.function.0,
                    module: frame.module.0,
                    package: frame.package.0,
                    path: frame.path.0,
                    in_app: frame.in_app,
                };
                Ok(frame)
            })
            .collect::<PyResult<_>>()?;

        let exception_data = enhancers::ExceptionData {
            ty: exception_data.ty.0,
            value: exception_data.value.0,
            mechanism: exception_data.mechanism.0,
        };

        self.0
            .apply_modifications_to_frames(&mut frames, &exception_data);

        let result = frames
            .into_iter()
            .map(|f| frame_to_dict(py, f))
            .collect::<PyResult<_>>()?;

        Ok(result)
    }
}

fn frame_to_dict(py: Python, frame: enhancers::Frame) -> PyResult<PyObject> {
    use enhancers::StringField;

    let obj = PyDict::new(py);
    obj.set_item("category", frame.category.as_ref().map(StringField::as_str))?;
    obj.set_item("family", frame.family.as_ref().map(StringField::as_str))?;
    obj.set_item("function", frame.function.as_ref().map(StringField::as_str))?;
    obj.set_item("module", frame.module.as_ref().map(StringField::as_str))?;

    obj.set_item("package", frame.package.as_ref().map(StringField::as_str))?;
    obj.set_item("path", frame.path.as_ref().map(StringField::as_str))?;
    obj.set_item("in_app", frame.in_app)?;

    Ok(obj.into())
}
