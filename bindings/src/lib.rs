use pyo3::prelude::*;

mod enhancers;
mod proguard;

macro_rules! add_module {
    ($py:ident : $parent:ident . $name:ident) => {
        // This uses the workaround from:
        // https://github.com/PyO3/pyo3/issues/759#issuecomment-1811992321
        // For the problem raised in https://pyo3.rs/v0.20.0/module#python-submodules
        {
            let module = PyModule::new($py, stringify!($name))?;
            $parent.add_submodule(module)?;
            let qualified_name = concat!("sentry_ophio.", stringify!($name));
            $py.import("sys")?
                .getattr("modules")?
                .set_item(qualified_name, module)?;
            module.setattr("__name__", qualified_name)?;
            module
        }
    };
}

/// A Python module implemented in Rust.
#[pymodule]
fn sentry_ophio(py: Python, m: &PyModule) -> PyResult<()> {
    let proguard_module = add_module!(py: m.proguard);
    proguard_module.add_class::<proguard::JavaStackFrame>()?;
    proguard_module.add_class::<proguard::ProguardMapper>()?;

    let enhancers_module = add_module!(py: m.enhancers);
    enhancers_module.add_class::<enhancers::Enhancements>()?;

    Ok(())
}
