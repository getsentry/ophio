//! Python bindings for the enhancers module.
//!
//! See `enhancers.pyi` for documentation on classes and functions.

use pyo3::prelude::*;
use pyo3::types::PyIterator;
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
    in_app: Option<bool>,
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
    #[staticmethod]
    fn empty() -> Self {
        Self(enhancers::Enhancements::default())
    }

    #[staticmethod]
    fn parse(input: &str, cache: &mut Cache) -> PyResult<Self> {
        let inner = enhancers::Enhancements::parse(input, &mut cache.0)?;
        Ok(Self(inner))
    }

    #[staticmethod]
    fn from_config_structure(input: &[u8], cache: &mut Cache) -> PyResult<Self> {
        let inner = enhancers::Enhancements::from_config_structure(input, &mut cache.0)?;
        Ok(Self(inner))
    }

    fn extend_from(&mut self, other: &Self) {
        self.0.extend_from(&other.0)
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
                    family: enhancers::Families::new(frame.family.0.as_deref().unwrap_or("other")),
                    function: frame.function.0,
                    module: frame.module.0,
                    package: frame.package.0,
                    path: frame.path.0,

                    in_app: frame.in_app,
                    in_app_last_changed: None,
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
            .map(|f| (f.category.as_ref().map(|c| c.as_str()), f.in_app).into_py(py))
            .collect();

        Ok(result)
    }
}
