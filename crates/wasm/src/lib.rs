//! WebAssembly bindings: JSON strings in and out of the core, nothing else.
//! Errors come back as thrown strings.

#![forbid(unsafe_code)]

use thesisgrid_core::api;
use wasm_bindgen::prelude::*;

fn js(r: Result<String, String>) -> Result<String, JsValue> {
    r.map_err(|e| JsValue::from_str(&e))
}

/// Runs a scenario (JSON) and returns the grids and summaries (JSON).
#[wasm_bindgen]
pub fn compare(scenario: &str) -> Result<String, JsValue> {
    js(api::compare(scenario))
}

/// [`compare`] for the page: the result JSON without its cells, plus the
/// cells as one Float64Array (see `thesisgrid_core::api::compare_packed`).
#[wasm_bindgen]
pub struct Packed {
    json: String,
    cells: Vec<f64>,
}

#[wasm_bindgen]
impl Packed {
    /// The result JSON, with every column's `cells` empty.
    #[wasm_bindgen(getter)]
    pub fn json(&self) -> String {
        self.json.clone()
    }

    /// Ten numbers per cell, column by column, row by row, date by date.
    #[wasm_bindgen(getter)]
    pub fn cells(&self) -> Vec<f64> {
        self.cells.clone()
    }
}

/// Runs a scenario and returns the result in packed form.
#[wasm_bindgen]
pub fn compare_packed(scenario: &str) -> Result<Packed, JsValue> {
    api::compare_packed(scenario)
        .map(|(json, cells)| Packed { json, cells })
        .map_err(|e| JsValue::from_str(&e))
}

/// Parses a quick-entry phrase against an as-of date (YYYY-MM-DD).
#[wasm_bindgen]
pub fn parse(text: &str, as_of: &str) -> Result<String, JsValue> {
    js(api::parse(text, as_of))
}

/// Venue presets and strategy names (JSON).
#[wasm_bindgen]
pub fn presets() -> String {
    api::presets()
}

/// The legs of a named option strategy (JSON).
#[wasm_bindgen]
pub fn strategy_legs(name: &str, strikes: Vec<f64>) -> Result<String, JsValue> {
    js(api::strategy_legs(name, &strikes))
}

/// A scenario as a shareable page link.
#[wasm_bindgen]
pub fn to_url(scenario: &str, base: Option<String>) -> Result<String, JsValue> {
    js(api::to_url(scenario, base.as_deref()))
}

/// A shared link back to scenario JSON.
#[wasm_bindgen]
pub fn from_url(link: &str) -> Result<String, JsValue> {
    js(api::from_url(link))
}

/// A scenario's grids and summary as CSV.
#[wasm_bindgen]
pub fn csv(scenario: &str) -> Result<String, JsValue> {
    js(api::csv(scenario))
}
