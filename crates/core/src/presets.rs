//! Venue presets, each with its source and as-of date. Every value can be
//! overridden by the user.

use crate::fees::BinaryFees;
use crate::fees::{
    hyperliquid::{hip3_rate, Hip4Fees},
    kalshi::KalshiFees,
    polymarket::PolymarketFees,
};
use crate::thesis::AssetClass;
use serde::{Deserialize, Serialize};

/// The date the preset values were checked against the venues' docs.
pub const AS_OF: &str = "2026-11-24";

/// A venue.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Venue {
    /// Kalshi.
    Kalshi,
    /// Polymarket (Polymarket Perps for perps).
    Polymarket,
    /// Hyperliquid (HIP-4 for binaries, HIP-3 builder markets for non-crypto perps).
    Hyperliquid,
    /// No preset: every value typed by the user.
    #[default]
    Custom,
}

/// Perp parameters a venue publishes.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PerpPreset {
    /// Venue.
    pub venue: Venue,
    /// Asset class the preset applies to.
    pub asset: AssetClass,
    /// Venue maximum leverage, when published for this asset class.
    pub max_leverage: Option<f64>,
    /// Maintenance margin rate as `factor / max_leverage`, when published.
    pub mm_factor: Option<f64>,
    /// Taker fee rate on notional (entry tier).
    pub taker_fee: f64,
    /// Maker fee rate on notional (entry tier).
    pub maker_fee: f64,
    /// Hours the entered funding rate is quoted for.
    pub funding_period_hours: f64,
    /// Hours between funding payments.
    pub funding_interval_hours: f64,
    /// Largest absolute rate per payment, if any.
    pub funding_cap: Option<f64>,
    /// Absolute rates below this per payment are set to zero, if any.
    pub funding_floor: Option<f64>,
    /// Default funding rate per period (the venue formula at zero premium).
    pub funding_rate: f64,
    /// Caveats shown with the preset.
    pub notes: Vec<&'static str>,
    /// Where each value comes from.
    pub sources: Vec<&'static str>,
}

const HL_FUNDING: &str = "https://hyperliquid.gitbook.io/hyperliquid-docs/trading/funding";
const HL_SPECS: &str =
    "https://hyperliquid.gitbook.io/hyperliquid-docs/trading/contract-specifications";
const HL_FEES: &str = "https://hyperliquid.gitbook.io/hyperliquid-docs/trading/fees";
const PM_FUNDING: &str = "https://docs.polymarket.com/perps/learn-about-trading/funding.md";
const PM_FEES: &str = "https://docs.polymarket.com/perps/learn-about-trading/fees.md";
const PM_MARGIN: &str = "https://docs.polymarket.com/perps/learn-about-trading/margin.md";
const PM_FAQ: &str = "https://docs.polymarket.com/perps/faq";
const K_PERPS: &str = "https://help.kalshi.com/en/articles/14679970-what-are-perpetual-futures";
const K_STOCKS: &str =
    "https://help.kalshi.com/en/articles/18183683-single-stock-perpetual-futures-on-kalshi";
const K_FEES: &str = "https://www.cftc.gov/filings/orgrules/rules10182634487.pdf";

/// The perp preset for a venue and asset class; `None` for [`Venue::Custom`].
pub fn perp(venue: Venue, asset: AssetClass) -> Option<PerpPreset> {
    let stocklike = matches!(asset, AssetClass::Stock | AssetClass::Index);
    let p = match venue {
        Venue::Custom => return None,
        Venue::Kalshi => PerpPreset {
            venue,
            asset,
            max_leverage: (asset == AssetClass::Crypto).then_some(6.0),
            mm_factor: None,
            taker_fee: 0.0004,
            maker_fee: 0.0002,
            funding_period_hours: if stocklike { 24.0 } else { 8.0 },
            funding_interval_hours: if stocklike { 24.0 } else { 8.0 },
            funding_cap: (!stocklike).then_some(0.02),
            funding_floor: (!stocklike).then_some(0.0001),
            funding_rate: 0.0,
            notes: kalshi_notes(asset),
            sources: vec![if stocklike { K_STOCKS } else { K_PERPS }, K_FEES],
        },
        Venue::Polymarket => {
            let crypto = asset == AssetClass::Crypto;
            PerpPreset {
                venue,
                asset,
                max_leverage: Some(if asset == AssetClass::Stock {
                    10.0
                } else {
                    20.0
                }),
                mm_factor: Some(0.5),
                taker_fee: 0.0004,
                maker_fee: 0.000_125,
                funding_period_hours: 8.0,
                funding_interval_hours: 1.0,
                funding_cap: Some(0.04),
                funding_floor: None,
                funding_rate: if crypto { 0.0001 } else { 0.000_05 },
                notes: vec![
                    "Max leverage is set per market; the docs' FAQ gives up to 20x for crypto, \
                     the S&P 500, oil, gold and silver, and up to 10x for other real-world assets.",
                    "Funding scale is 1.0 for crypto and 0.5 otherwise; the default rate is the \
                     docs' formula at zero premium.",
                ],
                sources: vec![PM_FEES, PM_MARGIN, PM_FUNDING, PM_FAQ],
            }
        }
        Venue::Hyperliquid => PerpPreset {
            venue,
            asset,
            max_leverage: match asset {
                AssetClass::Crypto => Some(40.0),
                AssetClass::Stock => Some(10.0),
                _ => None,
            },
            mm_factor: Some(0.5),
            taker_fee: hl_fee(asset, HL_TAKER),
            maker_fee: hl_fee(asset, HL_MAKER),
            funding_period_hours: 8.0,
            funding_interval_hours: 1.0,
            funding_cap: Some(0.005),
            funding_floor: None,
            funding_rate: 0.0001,
            notes: hyperliquid_notes(asset),
            sources: vec![HL_FEES, HL_SPECS, HL_FUNDING],
        },
    };
    Some(p)
}

fn kalshi_notes(asset: AssetClass) -> Vec<&'static str> {
    let mut n = vec![
        "Kalshi does not publish a maintenance margin rate; liquidation is shown where the \
         margin is used up until you enter one.",
        "Fees are the tier-0 rates in Kalshi's CFTC perps fee filing.",
    ];
    match asset {
        AssetClass::Crypto => n.push("Max leverage is 6x per Kalshi's contract specs."),
        AssetClass::Commodity => n.push(
            "Kalshi's gold and silver perp specs were not checked; funding follows Kalshi's \
             general 8-hour rules and leverage is left to you.",
        ),
        _ => n.push(
            "Stock perps fund once a day at 4:00 PM ET and leverage varies by stock (shown on \
             each market page); no funding cap or rounding rule is published for them.",
        ),
    }
    n
}

/// Base-tier (tier 0) perp rates for validator-operated perps.
const HL_TAKER: f64 = 0.000_45;
const HL_MAKER: f64 = 0.000_15;

/// Stock perps use xyz:NVDA's fees (deployer scale 1, growth mode on); crypto
/// uses the base tier; index and commodity builder markets keep the base tier
/// until checked.
fn hl_fee(asset: AssetClass, base: f64) -> f64 {
    if asset == AssetClass::Stock {
        hip3_rate(base, 1.0, true)
    } else {
        base
    }
}

fn hyperliquid_notes(asset: AssetClass) -> Vec<&'static str> {
    if asset == AssetClass::Crypto {
        return vec![
            "Max leverage is set per market, from 3x to 40x; 40x is the top of the range.",
            "Fees are the base-tier rates for validator-operated perps.",
        ];
    }
    if asset == AssetClass::Stock {
        return vec![
            "Stock perps are HIP-3 builder markets. Fees are xyz:NVDA's as shown on its market \
             page on Nov 24, 2026 (0.009% taker, 0.003% maker): the base tier times 2 for a \
             deployer fee scale of 1, less 90% for growth mode, per Hyperliquid's fee docs. \
             Other builder markets may differ.",
            "Max leverage 10x is xyz:NVDA's as of Nov 24, 2026; each builder market sets its own.",
        ];
    }
    vec![
        "Index and commodity perps are HIP-3 builder markets: fees depend on the deployer's \
         fee scale and growth mode, so the base-tier rates shown may not match.",
        "Each builder market sets its own max leverage; enter yours.",
    ]
}

/// A binary venue's default fee model and caveats.
pub fn binary(venue: Venue) -> (BinaryFees, Vec<&'static str>) {
    match venue {
        Venue::Kalshi => (BinaryFees::Kalshi(KalshiFees::default()), vec![]),
        Venue::Polymarket => (
            BinaryFees::Polymarket(PolymarketFees {
                rate: 0.0,
                exponent: 1.0,
                taker_only: true,
            }),
            vec![
                "Polymarket sets the fee rate per market (its feeSchedule); 0 is used until you \
                 enter the market's rate.",
            ],
        ),
        Venue::Hyperliquid => (
            BinaryFees::Hip4(Hip4Fees::default()),
            vec!["HIP-4 outcome fees are currently zero for initial testing per Hyperliquid."],
        ),
        Venue::Custom => (BinaryFees::Custom(Default::default()), vec![]),
    }
}

/// Every perp preset, for the page's dropdowns.
pub fn all_perps() -> Vec<PerpPreset> {
    let assets = [
        AssetClass::Crypto,
        AssetClass::Stock,
        AssetClass::Index,
        AssetClass::Commodity,
    ];
    let venues = [Venue::Kalshi, Venue::Polymarket, Venue::Hyperliquid];
    venues
        .iter()
        .flat_map(|&v| assets.iter().filter_map(move |&a| perp(v, a)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documented_values() {
        let pm = perp(Venue::Polymarket, AssetClass::Crypto).unwrap();
        // PRD acceptance: 20x market -> 0.5 / 20 = 2.5% maintenance margin.
        assert_eq!(pm.mm_factor.unwrap() / pm.max_leverage.unwrap(), 0.025);
        let k = perp(Venue::Kalshi, AssetClass::Stock).unwrap();
        assert_eq!((k.funding_interval_hours, k.funding_cap), (24.0, None));
        let k = perp(Venue::Kalshi, AssetClass::Commodity).unwrap();
        assert_eq!(k.funding_cap, Some(0.02));
        let hl = perp(Venue::Hyperliquid, AssetClass::Index).unwrap();
        assert_eq!(hl.max_leverage, None);
        assert_eq!(
            perp(Venue::Hyperliquid, AssetClass::Stock)
                .unwrap()
                .max_leverage,
            Some(10.0)
        );
        let hs = perp(Venue::Hyperliquid, AssetClass::Stock).unwrap();
        assert!((hs.taker_fee - 0.000_09).abs() < 1e-15);
        assert!((hs.maker_fee - 0.000_03).abs() < 1e-15);
        assert_eq!(hl.taker_fee, 0.000_45);
        assert!(perp(Venue::Custom, AssetClass::Crypto).is_none());
        assert_eq!(all_perps().len(), 12);
        for v in [
            Venue::Kalshi,
            Venue::Polymarket,
            Venue::Hyperliquid,
            Venue::Custom,
        ] {
            let _ = binary(v);
        }
    }
}
