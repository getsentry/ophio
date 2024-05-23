use pyo3::prelude::*;

mod enhancers;
mod proguard;

#[pymodule]
fn _bindings(_py: Python, m: Bound<PyModule>) -> PyResult<()> {
    m.add_class::<enhancers::Cache>()?;
    m.add_class::<enhancers::Component>()?;
    m.add_class::<enhancers::Enhancements>()?;
    m.add_class::<enhancers::AssembleResult>()?;
    m.add_class::<proguard::JavaStackFrame>()?;
    m.add_class::<proguard::ProguardMapper>()?;

    Ok(())
}
