//! Python bindings: JSON strings in and out of the core, nothing else.
//! Errors are raised as `ValueError`.

#![forbid(unsafe_code)]

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use thesisgrid_core::api;

fn py(r: Result<String, String>) -> PyResult<String> {
    r.map_err(PyValueError::new_err)
}

/// Runs a scenario (JSON) and returns the grids and summaries (JSON).
#[pyfunction]
fn compare(scenario: &str) -> PyResult<String> {
    py(api::compare(scenario))
}

/// Parses a quick-entry phrase against an as-of date (YYYY-MM-DD).
#[pyfunction]
fn parse(text: &str, as_of: &str) -> PyResult<String> {
    py(api::parse(text, as_of))
}

/// Venue presets and strategy names (JSON).
#[pyfunction]
fn presets() -> String {
    api::presets()
}

/// The legs of a named option strategy (JSON).
#[pyfunction]
fn strategy_legs(name: &str, strikes: Vec<f64>) -> PyResult<String> {
    py(api::strategy_legs(name, &strikes))
}

/// A scenario as a shareable page link.
#[pyfunction]
#[pyo3(signature = (scenario, base=None))]
fn to_url(scenario: &str, base: Option<&str>) -> PyResult<String> {
    py(api::to_url(scenario, base))
}

/// A shared link back to scenario JSON.
#[pyfunction]
fn from_url(link: &str) -> PyResult<String> {
    py(api::from_url(link))
}

/// A scenario's grids and summary as CSV.
#[pyfunction]
fn csv(scenario: &str) -> PyResult<String> {
    py(api::csv(scenario))
}

/// The compiled core.
#[pymodule]
fn _thesisgrid(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compare, m)?)?;
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(presets, m)?)?;
    m.add_function(wrap_pyfunction!(strategy_legs, m)?)?;
    m.add_function(wrap_pyfunction!(to_url, m)?)?;
    m.add_function(wrap_pyfunction!(from_url, m)?)?;
    m.add_function(wrap_pyfunction!(csv, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
