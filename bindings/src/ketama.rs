use pyo3::prelude::*;
use pyo3::types::PyList;
use rust_ophio::ketama;

#[pyclass]
pub struct KetamaPool(ketama::KetamaPool);

#[pymethods]
impl KetamaPool {
    #[new]
    fn new(keys: Bound<'_, PyList>) -> PyResult<Self> {
        let keys = keys
            .into_iter()
            .map(|k| k.extract())
            .collect::<PyResult<Vec<String>>>()?;
        let str_keys: Vec<&str> = keys.iter().map(|k| k.as_str()).collect();

        Ok(Self(ketama::KetamaPool::new(&str_keys)))
    }

    fn get_slot(&self, key: &str) -> usize {
        self.0.get_slot(key)
    }

    fn add_node(&mut self, server: &str) {
        self.0.add_node(server)
    }

    fn remove_node(&mut self, server: &str) {
        self.0.remove_node(server)
    }
}
