//! Python bindings for glob pattern matching.

use pyo3::prelude::*;
use rust_ophio::glob::{self, GlobOptions};

#[pyfunction]
#[pyo3(signature = (value, pat, double_star=false, case_insensitive=false, path_normalize=false, allow_newline=false))]
pub fn is_glob_match(
    value: &[u8],
    pat: &str,
    double_star: bool,
    case_insensitive: bool,
    path_normalize: bool,
    allow_newline: bool,
) -> bool {
    glob::is_glob_match(
        value,
        pat,
        GlobOptions {
            double_star,
            case_insensitive,
            path_normalize,
            allow_newline,
        },
    )
}
