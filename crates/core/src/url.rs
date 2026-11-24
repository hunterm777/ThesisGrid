//! Scenario <-> link encoding. The whole scenario lives in the URL fragment
//! as base64url JSON, so no server is involved.

use crate::scenario::Scenario;
use crate::{err, Res};

/// The page's address.
pub const PAGE_URL: &str = "https://hunterm777.github.io/ThesisGrid/";

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// base64url without padding.
pub fn b64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..=chunk.len() {
            out.push(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize] as char);
        }
    }
    out
}

/// Decodes base64url, with or without padding.
pub fn b64_decode(s: &str) -> Res<Vec<u8>> {
    let mut vals = Vec::with_capacity(s.len());
    for c in s.trim_end_matches('=').bytes() {
        match ALPHABET.iter().position(|&a| a == c) {
            Some(v) => vals.push(v as u32),
            None => return err("the link is not a ThesisGrid scenario"),
        }
    }
    if vals.len() % 4 == 1 {
        return err("the link is cut short");
    }
    let mut out = Vec::with_capacity(vals.len() * 3 / 4);
    for chunk in vals.chunks(4) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, v)| n | (v << (18 - 6 * i)));
        for i in 0..chunk.len() - 1 {
            out.push((n >> (16 - 8 * i)) as u8);
        }
    }
    Ok(out)
}

/// The scenario as a page link (`base` defaults to [`PAGE_URL`]).
pub fn to_url(sc: &Scenario, base: Option<&str>) -> String {
    let json = serde_json::to_string(sc).expect("scenario serializes");
    format!(
        "{}#s={}",
        base.unwrap_or(PAGE_URL),
        b64_encode(json.as_bytes())
    )
}

/// Reads a scenario from a page link or a bare `s=` fragment.
pub fn from_url(link: &str) -> Res<Scenario> {
    let payload = match link.rfind("s=") {
        Some(i) => &link[i + 2..],
        None => link,
    };
    let bytes = b64_decode(payload.trim())?;
    let text =
        String::from_utf8(bytes).map_err(|_| crate::Error("the link is not UTF-8".into()))?;
    serde_json::from_str(&text)
        .map_err(|e| crate::Error(format!("the link's scenario is invalid: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trips() {
        for s in [
            "",
            "f",
            "fo",
            "foo",
            "foob",
            "fooba",
            "foobar",
            "\u{1F600} ünï",
        ] {
            let e = b64_encode(s.as_bytes());
            assert!(!e.contains('='));
            assert_eq!(b64_decode(&e).unwrap(), s.as_bytes());
        }
        assert_eq!(b64_encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(b64_decode("Zm9vYg==").unwrap(), b"foob");
        assert!(b64_decode("a").is_err());
        assert!(b64_decode("a*b").is_err());
    }

    #[test]
    fn scenario_round_trips() {
        let sc: Scenario = serde_json::from_str(
            r#"{"thesis":{"spot":95000,"target":110000,"target_date":"2026-12-31",
                "as_of":"2026-11-24"},"instruments":[{"kind":"spot"}]}"#,
        )
        .unwrap();
        let link = to_url(&sc, None);
        assert!(link.starts_with(PAGE_URL));
        assert_eq!(from_url(&link).unwrap(), sc);
        assert!(from_url("https://x/#s=AAAA").is_err());
        assert!(from_url(&b64_encode(&[0xff, 0xfe])).is_err());
    }
}
