use pyo3::prelude::*;

pub mod enhancers;
mod proguard;

/// A Python module implemented in Rust.
#[pymodule]
fn sentry_ophio(py: Python, m: &PyModule) -> PyResult<()> {
    // FIXME: https://pyo3.rs/v0.20.0/module#python-submodules
    let proguard_module = PyModule::new(py, "proguard")?;
    m.add_submodule(proguard_module)?;
    py.import("sys")?
        .getattr("modules")?
        .set_item("sentry_ophio.proguard", proguard_module)?;
    proguard_module.setattr("__name__", "sentry_ophio.proguard")?;

    proguard_module.add_class::<proguard::JavaStackFrame>()?;
    proguard_module.add_class::<proguard::ProguardMapper>()?;

    Ok(())
}
