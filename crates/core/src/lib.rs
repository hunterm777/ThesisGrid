//! ThesisGrid pricing core.
//!
//! All pricing, sizing and fee math lives here; the WebAssembly and Python
//! wrappers only move JSON in and out. The crate does no I/O.
//!
//! Every transcendental function goes through [`libm`], so the native, WASM
//! and Python builds produce bit-identical numbers.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod api;
pub mod binary;
pub mod dates;
pub mod fees;
pub mod format;
pub mod grid;
pub mod instrument;
pub mod iv;
pub mod math;
pub mod option;
pub mod parse;
pub mod perp;
pub mod presets;
pub mod scenario;
pub mod sizing;
pub mod spot;
pub mod strategies;
pub mod summary;
pub mod thesis;
pub mod url;

pub use dates::Date;
pub use scenario::{compare, compare_json, CompareResult, Scenario};

use serde::{Deserialize, Serialize};
use std::fmt;

/// Long or short.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    /// Profits when the price rises.
    #[default]
    Long,
    /// Profits when the price falls.
    Short,
}

impl Side {
    /// +1 for long, -1 for short.
    pub fn sign(self) -> f64 {
        match self {
            Side::Long => 1.0,
            Side::Short => -1.0,
        }
    }
}

/// Which side of the book an order takes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Liquidity {
    /// Fills immediately against a resting order.
    #[default]
    Taker,
    /// Rests on the book.
    Maker,
}

/// An input the core cannot work with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// Result type used across the crate.
pub type Res<T> = Result<T, Error>;

/// Builds an [`Error`] from anything printable.
pub fn err<T>(msg: impl Into<String>) -> Res<T> {
    Err(Error(msg.into()))
}
