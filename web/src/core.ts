// The only door to the math: every number on the page comes from the Rust
// core, compiled to WebAssembly and inlined into this bundle (no fetch).
import { initSync, compare_packed as wPacked, parse as wParse, presets as wPresets,
  to_url as wToUrl, from_url as wFromUrl, csv as wCsv } from "./wasm/thesisgrid_wasm.js";
import wasmBase64 from "./wasm/thesisgrid_wasm_bg.wasm?b64";
import type { Cell, CompareResult, Parsed, Presets, Scenario } from "./types";

let ready = false;

/** Compiles the inlined module; call once before anything else. */
export function init(): void {
  if (ready) return;
  const bin = atob(wasmBase64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  initSync({ module: bytes });
  ready = true;
}

const text = (e: unknown): string => (typeof e === "string" ? e : String(e));

/** Numbers per cell in the packed array (see the core's `compare_packed`). */
const FIELDS = 10;

/** Runs the scenario. The cells cross from WASM as one Float64Array rather
 * than JSON text, which keeps a full redraw inside a frame. */
export function compare(sc: Scenario): { ok: CompareResult } | { err: string } {
  try {
    const packed = wPacked(JSON.stringify(sc));
    const r = JSON.parse(packed.json) as CompareResult;
    const f = packed.cells;
    packed.free();
    const rows = r.prices.length;
    const cols = r.dates.length;
    let k = 0;
    for (const c of r.columns) {
      if (c.error) continue;
      const grid: Cell[][] = [];
      for (let i = 0; i < rows; i++) {
        const line: Cell[] = [];
        for (let j = 0; j < cols; j++, k += FIELDS) {
          line.push({
            pnl: f[k] ?? 0, pct: f[k + 1] ?? 0, value: f[k + 2] ?? 0, fees: f[k + 3] ?? 0, funding: f[k + 4] ?? 0,
            delta: f[k + 5] ?? 0, gamma: f[k + 6] ?? 0, theta: f[k + 7] ?? 0, vega: f[k + 8] ?? 0, liquidated: f[k + 9] === 1,
          });
        }
        grid.push(line);
      }
      c.cells = grid;
    }
    if (k !== f.length) throw new Error("packed cells do not match the grid");
    return { ok: r };
  } catch (e) {
    return { err: text(e) };
  }
}

export function parse(phrase: string, asOf: string): Parsed {
  return JSON.parse(wParse(phrase, asOf)) as Parsed;
}

export function presets(): Presets {
  return JSON.parse(wPresets()) as Presets;
}

export function toUrl(sc: Scenario, base: string): string {
  return wToUrl(JSON.stringify(sc), base);
}

export function fromUrl(link: string): Scenario {
  return JSON.parse(wFromUrl(link)) as Scenario;
}

export function csv(sc: Scenario): string {
  return wCsv(JSON.stringify(sc));
}
