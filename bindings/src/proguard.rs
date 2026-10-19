use memmap2::Mmap;
use pyo3::prelude::*;
use pyo3::types::{IntoPyDict, PyBytes};
use yoke::Yoke;

#[pyclass(frozen, get_all)]
pub struct JavaStackFrame {
    pub class_name: String,
    pub method: String,
    pub line: Option<usize>,
    pub file: Option<String>,
    pub parameters: Option<String>,
}

#[derive(yoke::Yokeable)]
struct Inner<'a> {
    mapping: proguard::ProguardMapping<'a>,
    mapper: proguard::ProguardMapper<'a>,
}

#[pyclass(frozen)]
pub struct ProguardMapper {
    inner: Yoke<Inner<'static>, Mmap>,
}

#[pymethods]
impl ProguardMapper {
    #[staticmethod]
    #[pyo3(signature = (path, initialize_param_mapping = false))]
    pub fn open(path: &str, initialize_param_mapping: bool) -> PyResult<Self> {
        let file = std::fs::File::open(path)?;
        let mmap = unsafe { Mmap::map(&file) }?;

        let inner = Yoke::attach_to_cart(mmap, |data| {
            let mapping = proguard::ProguardMapping::new(data);
            let mapper =
                proguard::ProguardMapper::new_with_param_mapping(mapping, initialize_param_mapping);
            Inner { mapping, mapper }
        });

        Ok(ProguardMapper { inner })
    }

    /// Returns the UUID of the file.
    #[getter]
    pub fn uuid<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        let uuid = self.inner.get().mapping.uuid();

        let bytes = PyBytes::new(py, uuid.as_bytes());
        let kwargs = [("bytes", bytes)].into_py_dict(py)?;
        py.import("uuid")?.getattr("UUID")?.call((), Some(&kwargs))
    }

    /// True if the file contains line information.
    #[getter]
    pub fn has_line_info(&self) -> bool {
        self.inner.get().mapping.has_line_info()
    }

    /// Remaps the given class name.
    pub fn remap_class(&self, klass: &str) -> Option<&str> {
        self.inner.get().mapper.remap_class(klass)
    }

    /// Remaps the given class and method name if that can be done unambiguously.
    pub fn remap_method(&self, klass: &str, method: &str) -> Option<(&str, &str)> {
        self.inner.get().mapper.remap_method(klass, method)
    }

    #[pyo3(signature = (klass, method, line, parameters=None))]
    pub fn remap_frame(
        &self,
        klass: &str,
        method: &str,
        line: usize,
        parameters: Option<&str>,
    ) -> Vec<JavaStackFrame> {
        let frame = if let Some(parameters) = parameters {
            proguard::StackFrame::with_parameters(klass, method, parameters)
        } else {
            proguard::StackFrame::new(klass, method, line)
        };

        self.inner
            .get()
            .mapper
            .remap_frame(&frame)
            .map(|frame| JavaStackFrame {
                class_name: frame.class().into(),
                method: frame.method().into(),
                file: frame.file().map(Into::into),
                line: frame.line(),
                parameters: frame.parameters().map(|s| s.to_owned()),
            })
            .collect()
    }
}
