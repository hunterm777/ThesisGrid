//! M4: the shared scenario suite. Each `fixtures/scenarios/*.json` runs here
//! and must match `fixtures/expected/*.json` to 1e-9; the WASM and Python
//! builds run the same files against the same expected outputs.
//!
//! Set `THESISGRID_BLESS=1` to rewrite the expected outputs.

mod common;

use common::{diff, fixtures};
use serde_json::Value;

#[test]
fn scenarios_match_expected() {
    let bless = std::env::var("THESISGRID_BLESS").is_ok();
    let mut n = 0;
    let mut names: Vec<_> = std::fs::read_dir(fixtures().join("scenarios"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    names.sort();
    for path in names {
        let input = std::fs::read_to_string(&path).unwrap();
        let got = thesisgrid_core::compare_json(&input).unwrap();
        let got: Value = serde_json::from_str(&got).unwrap();
        let out = fixtures().join("expected").join(path.file_name().unwrap());
        if bless {
            std::fs::write(&out, serde_json::to_string_pretty(&got).unwrap() + "\n").unwrap();
        }
        let want: Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        if let Some(d) = diff(&got, &want, path.file_stem().unwrap().to_str().unwrap()) {
            panic!("{d}");
        }
        n += 1;
    }
    assert!(n >= 7);
}
