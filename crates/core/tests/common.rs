//! Shared helpers for the fixture suites; each suite uses some of them.
#![allow(dead_code)]

use serde_json::Value;
use std::path::PathBuf;

/// The repository's `fixtures/` folder.
pub fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// True when two numbers agree to 1e-9 (relative above 1).
pub fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * b.abs().max(1.0)
}

/// Compares two JSON trees, numbers to 1e-9; returns the first difference.
pub fn diff(a: &Value, b: &Value, path: &str) -> Option<String> {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            (!close(x, y)).then(|| format!("{path}: {x} vs {y}"))
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                return Some(format!("{path}: length {} vs {}", x.len(), y.len()));
            }
            x.iter()
                .zip(y)
                .enumerate()
                .find_map(|(i, (p, q))| diff(p, q, &format!("{path}[{i}]")))
        }
        (Value::Object(x), Value::Object(y)) => {
            if x.len() != y.len() {
                return Some(format!("{path}: keys differ"));
            }
            x.iter().find_map(|(k, v)| match y.get(k) {
                Some(w) => diff(v, w, &format!("{path}.{k}")),
                None => Some(format!("{path}.{k} missing")),
            })
        }
        _ => (a != b).then(|| format!("{path}: {a} vs {b}")),
    }
}
