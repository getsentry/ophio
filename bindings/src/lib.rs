use pyo3::prelude::*;

mod enhancers;
mod proguard;

#[pymodule]
fn _bindings(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<proguard::JavaStackFrame>()?;
    m.add_class::<proguard::ProguardMapper>()?;
    m.add_class::<enhancers::Enhancements>()?;
    m.add_class::<enhancers::Cache>()?;

    Ok(())
}
