use pyo3::prelude::*;

mod codeowners;
mod enhancers;
mod glob;

#[pymodule]
fn _bindings(_py: Python, m: Bound<PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(codeowners::is_codeowners_path_match, &m)?)?;
    m.add_function(wrap_pyfunction!(glob::is_glob_match, &m)?)?;
    m.add_class::<enhancers::Cache>()?;
    m.add_class::<enhancers::Component>()?;
    m.add_class::<enhancers::Enhancements>()?;
    m.add_class::<enhancers::AssembleResult>()?;

    Ok(())
}
