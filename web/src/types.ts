// Shapes of the JSON the core reads and writes. The core is the source of
// truth: optional fields left out take the core's (or the preset's) default.

export type AssetClass = "crypto" | "stock" | "index" | "commodity";
export type Mode = "equal_dollars" | "equal_max_loss" | "equal_exposure" | "manual";
export type Venue = "kalshi" | "polymarket" | "hyperliquid" | "custom";
export type Side = "long" | "short";
export type Display = "dollars" | "percent";

export interface Thesis {
  underlying?: string;
  asset_class?: AssetClass;
  spot: number;
  target: number;
  target_date: string;
  as_of: string;
  touch?: "by" | "at";
  direction?: "bullish" | "bearish";
  rate?: number;
  iv?: number;
  commodity_day_count?: "calendar365" | "trading252";
}

export interface LegSpec {
  kind: "call" | "put" | "underlying";
  side?: Side;
  strike?: number;
  expiry?: string;
  premium?: number;
  iv?: number;
  ratio?: number;
}

/** Every instrument is a plain record the core validates; keys follow the core. */
export type Spec = { kind: "spot" | "perp" | "option" | "binary" } & Record<string, unknown>;

export interface PriceAxis {
  kind: "range_pct" | "range" | "list";
  lo?: number;
  hi?: number;
  steps?: number;
  values?: number[];
}

export interface DateAxis {
  start?: string;
  end?: string;
  step?: number;
  max_cols?: number;
  list?: string[];
}

export interface Scenario {
  v?: number;
  thesis: Thesis;
  instruments: Spec[];
  mode?: Mode;
  budget?: number;
  grid?: { prices?: PriceAxis; dates?: DateAxis; display?: Display };
}

export interface Cell {
  pnl: number;
  pct: number;
  value: number;
  fees: number;
  funding: number;
  delta: number;
  gamma: number;
  theta: number;
  vega: number;
  liquidated: boolean;
}

export interface Summary {
  qty: number;
  cost: number;
  max_loss: number | null;
  max_gain: number | null;
  breakevens: number[];
  pnl_at_target: number;
  roc_at_target: number;
  liquidation: number | null;
  dollar_delta: number;
}

export interface Column {
  label: string;
  kind: Spec["kind"];
  venue: Venue | null;
  error: string | null;
  notes: string[];
  summary: Summary | null;
  cells: Cell[][];
}

export interface CompareResult {
  prices: number[];
  dates: string[];
  target_row: number;
  target_col: number;
  mode: Mode;
  budget: number;
  display: Display;
  sizing_note: string;
  columns: Column[];
  presets_as_of: string;
}

export interface PerpPreset {
  venue: Venue;
  asset: AssetClass;
  max_leverage: number | null;
  mm_factor: number | null;
  taker_fee: number;
  maker_fee: number;
  funding_period_hours: number;
  funding_interval_hours: number;
  funding_cap: number | null;
  funding_floor: number | null;
  funding_rate: number;
  notes: string[];
  sources: string[];
}

export interface Presets {
  as_of: string;
  perps: PerpPreset[];
  binaries: { venue: Venue; fees: Record<string, unknown>; notes: string[] }[];
  strategies: string[];
}

export interface Parsed {
  underlying: string | null;
  asset_class: AssetClass | null;
  target: number | null;
  target_date: string | null;
  direction: "bullish" | "bearish" | null;
  touch: "by" | "at" | null;
  missing: string[];
}
