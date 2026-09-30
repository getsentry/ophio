//! Python bindings for CODEOWNERS path pattern matching.

use pyo3::prelude::*;
use rust_ophio::codeowners;

#[pyfunction]
pub fn is_codeowners_path_match(value: &[u8], pattern: &str) -> bool {
    codeowners::is_codeowners_path_match(value, pattern)
}
