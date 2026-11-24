// The page's single source of state: one Scenario object, the same JSON the
// Python package and the share link carry. Every edit recomputes in the core.
import * as core from "./core";
import { addDays, today } from "./format";
import type { CompareResult, Scenario } from "./types";

export const PAGE_URL = "https://hunterm777.github.io/ThesisGrid/";

/** An illustrative starting setup: every number is an input, not market data. */
export function defaultScenario(): Scenario {
  const asOf = today();
  const target = addDays(asOf, 37);
  return {
    v: 1,
    thesis: {
      underlying: "BTC",
      asset_class: "crypto",
      spot: 95000,
      target: 110000,
      target_date: target,
      as_of: asOf,
      rate: 0.04,
      iv: 0.55,
    },
    mode: "equal_max_loss",
    budget: 500,
    instruments: [
      { kind: "spot", side: "long", stop: 85000 },
      { kind: "perp", venue: "hyperliquid", side: "long", leverage: 5 },
      { kind: "option", strategy: "bull_call_spread", strikes: [100000, 115000], expiry: target },
      { kind: "binary", venue: "kalshi", side: "yes", strike: 110000, expiry: target, price: 0.2 },
    ],
  };
}

type Listener = (structural: boolean) => void;

class Store {
  sc: Scenario;
  result: CompareResult | null = null;
  error: string | null = null;
  linkError: string | null = null;
  /** Milliseconds for the last recompute plus redraw. */
  lastMs = 0;
  private listeners: Listener[] = [];
  private hashTimer: ReturnType<typeof setTimeout> | undefined;

  constructor() {
    this.sc = defaultScenario();
  }

  /** Loads the scenario from the page link, if it carries one. */
  loadFromHash(hash: string): void {
    if (!hash.includes("s=")) return;
    try {
      this.sc = core.fromUrl(hash);
      this.sc.instruments ??= [];
    } catch (e) {
      this.linkError = `This link's scenario could not be read (${String(e)}). Showing the example instead.`;
    }
  }

  subscribe(fn: Listener): void {
    this.listeners.push(fn);
  }

  /** Applies an edit and redraws. `structural` rebuilds the input cards too. */
  update(edit: (sc: Scenario) => void, structural = false): void {
    edit(this.sc);
    this.run(structural);
  }

  run(structural = true): void {
    const t0 = performance.now();
    const r = core.compare(this.sc);
    if ("ok" in r) {
      this.result = r.ok;
      this.error = null;
    } else {
      this.error = r.err;
    }
    for (const fn of this.listeners) fn(structural);
    this.lastMs = performance.now() - t0;
    document.body.dataset["renderMs"] = this.lastMs.toFixed(2);
    // The address bar follows the page, but after typing pauses, so a burst
    // of edits doesn't re-encode the link each time.
    clearTimeout(this.hashTimer);
    this.hashTimer = setTimeout(() => this.syncHash(), 300);
  }

  link(): string {
    return core.toUrl(this.sc, PAGE_URL);
  }

  private syncHash(): void {
    const here = location.href.split("#")[0] ?? "";
    const link = core.toUrl(this.sc, here);
    if (link !== location.href) history.replaceState(null, "", link);
  }
}

export const store = new Store();
