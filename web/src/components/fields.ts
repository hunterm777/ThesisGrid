// The inputs inside each instrument card, keyed to the core's field names.
// A blank field takes the venue preset's (or the core's) value, shown as the
// placeholder, so every preset value can be overridden and restored.
import * as core from "../core";
import { dateField, h, numField, selectField, show, textField } from "../dom";
import { store } from "../state";
import type { AssetClass, LegSpec, PerpPreset, Spec, Venue } from "../types";

const PRESETS = (() => {
  let p: ReturnType<typeof core.presets> | null = null;
  return () => (p ??= core.presets());
})();

const VENUES = [
  ["kalshi", "Kalshi"],
  ["polymarket", "Polymarket"],
  ["hyperliquid", "Hyperliquid"],
  ["custom", "Custom"],
] as const;
const SIDES = [
  ["long", "Long"],
  ["short", "Short"],
] as const;

/** Sets or clears one field and recomputes. */
function setter(spec: Spec, key: string, structural = false) {
  return (v: unknown): void =>
    store.update(() => {
      if (v === undefined || v === "") delete spec[key];
      else spec[key] = v;
    }, structural);
}

function num(spec: Spec, key: string, label: string, opts: { scale?: number; suffix?: string; ph?: number | null } = {}) {
  const ph = opts.ph === undefined || opts.ph === null ? undefined : show(opts.ph, opts.scale ?? 1);
  return numField({
    label,
    value: spec[key] as number | undefined,
    ...(opts.scale !== undefined ? { scale: opts.scale } : {}),
    ...(opts.suffix !== undefined ? { suffix: opts.suffix } : {}),
    ...(ph !== undefined ? { placeholder: ph } : {}),
    onValue: setter(spec, key),
  });
}

export function spotFields(spec: Spec): HTMLElement[] {
  const th = store.sc.thesis;
  return [
    selectField("Side", (spec["side"] as string) ?? "long", SIDES, setter(spec, "side")),
    num(spec, "price", "Entry price", { ph: th.spot }),
    num(spec, "fee_pct", "Fee", { scale: 100, suffix: "%", ph: 0 }),
    num(spec, "fee_fixed", "Fee per trade", { suffix: "$", ph: 0 }),
    num(spec, "stop", "Stop price"),
  ];
}

function perpPreset(venue: Venue, asset: AssetClass): PerpPreset | undefined {
  return PRESETS().perps.find((p) => p.venue === venue && p.asset === asset);
}

export function perpFields(spec: Spec): HTMLElement[] {
  const th = store.sc.thesis;
  const venue = (spec["venue"] as Venue) ?? "custom";
  const asset = (spec["asset"] as AssetClass) ?? th.asset_class ?? "crypto";
  const p = perpPreset(venue, asset);
  const mm = p?.mm_factor && p.max_leverage ? p.mm_factor / p.max_leverage : null;
  const out = [
    selectField("Venue", venue, VENUES, setter(spec, "venue", true)),
    selectField("Side", (spec["side"] as string) ?? "long", SIDES, setter(spec, "side")),
    num(spec, "leverage", "Leverage", { suffix: "x", ph: 1 }),
    num(spec, "entry", "Entry price", { ph: th.spot }),
    num(spec, "funding_rate", `Funding per ${p?.funding_period_hours ?? 8}h`, { scale: 100, suffix: "%", ph: p?.funding_rate ?? 0 }),
    num(spec, "funding_period_hours", "Rate period", { suffix: "h", ph: p?.funding_period_hours ?? 8 }),
    num(spec, "funding_interval_hours", "Paid every", { suffix: "h", ph: p?.funding_interval_hours ?? 8 }),
    num(spec, "maintenance_margin", "Maint. margin", { scale: 100, suffix: "%", ph: mm }),
    num(spec, "max_leverage", "Max leverage", { suffix: "x", ph: p?.max_leverage ?? null }),
    num(spec, "taker_fee", "Taker fee", { scale: 100, suffix: "%", ph: p?.taker_fee ?? 0 }),
    num(spec, "maker_fee", "Maker fee", { scale: 100, suffix: "%", ph: p?.maker_fee ?? 0 }),
    selectField("Order", (spec["liquidity"] as string) ?? "taker", [["taker", "Taker"], ["maker", "Maker"]] as const, setter(spec, "liquidity")),
  ];
  return out;
}

const LEG_KINDS = [
  ["call", "Call"],
  ["put", "Put"],
  ["underlying", "Underlying"],
] as const;

function legRow(spec: Spec, legs: LegSpec[], i: number): HTMLElement {
  const leg = legs[i] as LegSpec & Record<string, unknown>;
  const set = (key: string) => (v: unknown) =>
    store.update(() => {
      if (v === undefined || v === "") delete leg[key];
      else leg[key] = v;
      spec["legs"] = legs;
    });
  const n = (key: string, label: string, scale?: number) =>
    numField({ label, value: leg[key] as number | undefined, ...(scale ? { scale } : {}), onValue: set(key) });
  const remove = h("button", { type: "button", class: "ghost", "aria-label": `Remove leg ${i + 1}` }, "Remove");
  remove.addEventListener("click", () => {
    legs.splice(i, 1);
    store.update(() => (spec["legs"] = legs), true);
  });
  return h(
    "div",
    { class: "leg" },
    selectField("Leg", leg.kind, LEG_KINDS, set("kind")),
    selectField("Side", leg.side ?? "long", SIDES, set("side")),
    n("strike", "Strike"),
    n("premium", "Premium"),
    n("iv", "IV", 100),
    n("ratio", "Qty"),
    dateField("Expiry", leg.expiry, set("expiry")),
    remove,
  );
}

export function optionFields(spec: Spec): HTMLElement[] {
  const th = store.sc.thesis;
  const strategy = (spec["strategy"] as string) ?? "custom";
  const names = PRESETS().strategies.map((s) => [s, s.replaceAll("_", " ")] as const);
  const isStock = th.asset_class === "stock" || th.asset_class === "index";
  const out: HTMLElement[] = [
    selectField("Strategy", strategy, names, (v) => {
      store.update(() => {
        spec["strategy"] = v;
        if (v === "custom" && !Array.isArray(spec["legs"])) spec["legs"] = [{ kind: "call", strike: th.target }];
      }, true);
    }),
  ];
  if (strategy !== "custom") {
    const strikes = Array.isArray(spec["strikes"]) ? (spec["strikes"] as number[]).join(", ") : "";
    out.push(
      textField("Strikes", strikes, (v) => {
        const list = v.split(/[,\s]+/).filter(Boolean).map(Number);
        if (list.every(Number.isFinite)) store.update(() => (spec["strikes"] = list));
      }, "low to high"),
    );
  }
  out.push(
    dateField("Expiry", spec["expiry"] as string | undefined, setter(spec, "expiry")),
    num(spec, "iv", "IV", { scale: 100, suffix: "%", ph: th.iv ?? 0.5 }),
    num(spec, "fee_per_contract", "Fee / contract", { suffix: "$", ph: 0 }),
    num(spec, "multiplier", "Multiplier", { ph: isStock ? 100 : 1 }),
    selectField("Model", (spec["model"] as string) ?? "black_scholes", [["black_scholes", "Black-Scholes"], ["black76", "Black-76"]] as const, setter(spec, "model")),
  );
  if (strategy === "custom") {
    const legs = (Array.isArray(spec["legs"]) ? spec["legs"] : []) as LegSpec[];
    const add = h("button", { type: "button", class: "ghost" }, "Add leg");
    add.addEventListener("click", () => {
      legs.push({ kind: "call", strike: th.target });
      store.update(() => (spec["legs"] = legs), true);
    });
    out.push(h("div", { class: "legs" }, ...legs.map((_, i) => legRow(spec, legs, i)), add));
  }
  return out;
}

type Fees = Record<string, unknown> & { model?: string };

const FEE_MODEL: Record<Venue, string> = { kalshi: "kalshi", polymarket: "polymarket", hyperliquid: "hip4", custom: "custom" };

function feeField(spec: Spec, venue: Venue, key: string, label: string, scale = 1, suffix?: string): HTMLElement {
  const fees = (spec["fees"] as Fees | undefined) ?? {};
  const preset = PRESETS().binaries.find((b) => b.venue === venue)?.fees ?? {};
  const ph = typeof preset[key] === "number" ? show(preset[key] as number, scale) : undefined;
  return numField({
    label,
    value: fees.model === FEE_MODEL[venue] ? (fees[key] as number | undefined) : undefined,
    scale,
    ...(suffix ? { suffix } : {}),
    ...(ph !== undefined ? { placeholder: ph } : {}),
    onValue: (v) =>
      store.update(() => {
        const f: Fees = fees.model === FEE_MODEL[venue] ? { ...fees } : { model: FEE_MODEL[venue] };
        if (v === undefined) delete f[key];
        else f[key] = v;
        if (Object.keys(f).length === 1) delete spec["fees"];
        else spec["fees"] = f;
      }),
  });
}

export function binaryFields(spec: Spec): HTMLElement[] {
  const th = store.sc.thesis;
  const venue = (spec["venue"] as Venue) ?? "custom";
  const contract = (spec["contract"] as string) ?? "above";
  const out: HTMLElement[] = [
    selectField("Venue", venue, VENUES, (v) => {
      store.update(() => {
        spec["venue"] = v;
        delete spec["fees"];
      }, true);
    }),
    selectField("Side", (spec["side"] as string) ?? "yes", [["yes", "YES"], ["no", "NO"]] as const, setter(spec, "side")),
    selectField("Contract", contract, [["above", "Above"], ["range", "Range"]] as const, (v) => {
      store.update(() => (spec["contract"] = v), true);
    }),
  ];
  if (contract === "range") out.push(num(spec, "lower", "Lower"), num(spec, "upper", "Upper"));
  else out.push(num(spec, "strike", "Strike", { ph: th.target }));
  out.push(
    dateField("Expiry", spec["expiry"] as string | undefined, setter(spec, "expiry")),
    num(spec, "price", "Contract price", { suffix: "$" }),
    num(spec, "iv", "IV", { scale: 100, suffix: "%", ph: null }),
    selectField("Order", (spec["liquidity"] as string) ?? "taker", [["taker", "Taker"], ["maker", "Maker"]] as const, setter(spec, "liquidity")),
  );
  if (venue === "polymarket") out.push(feeField(spec, venue, "rate", "Fee rate", 100, "%"));
  if (venue === "hyperliquid") out.push(feeField(spec, venue, "taker_rate", "Taker fee", 100, "%"));
  if (venue === "custom") {
    out.push(feeField(spec, venue, "per_contract", "Fee / contract", 1, "$"));
    out.push(feeField(spec, venue, "pct_notional", "Fee", 100, "%"));
  }
  return out;
}
