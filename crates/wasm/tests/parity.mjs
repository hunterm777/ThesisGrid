// Runs the shared fixture suite against the WASM build (Node target).
// Usage: node crates/wasm/tests/parity.mjs <path to wasm-bindgen nodejs pkg>
import { readFileSync, readdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { createRequire } from "node:module";

const root = resolve(import.meta.dirname, "../../..");
const pkg = resolve(process.argv[2] ?? join(root, "crates/wasm/pkg-node"));
const tg = createRequire(import.meta.url)(join(pkg, "thesisgrid_wasm.js"));

const close = (a, b) => Math.abs(a - b) <= 1e-9 * Math.max(1, Math.abs(b));

function diff(a, b, path) {
  if (typeof a === "number" && typeof b === "number") {
    return close(a, b) ? null : `${path}: ${a} vs ${b}`;
  }
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) return `${path}: length ${a.length} vs ${b.length}`;
    for (let i = 0; i < a.length; i++) {
      const d = diff(a[i], b[i], `${path}[${i}]`);
      if (d) return d;
    }
    return null;
  }
  if (a && b && typeof a === "object" && typeof b === "object") {
    const ka = Object.keys(a);
    if (ka.length !== Object.keys(b).length) return `${path}: keys differ`;
    for (const k of ka) {
      if (!(k in b)) return `${path}.${k} missing`;
      const d = diff(a[k], b[k], `${path}.${k}`);
      if (d) return d;
    }
    return null;
  }
  return a === b ? null : `${path}: ${JSON.stringify(a)} vs ${JSON.stringify(b)}`;
}

const dir = join(root, "fixtures/scenarios");
const names = readdirSync(dir).filter((f) => f.endsWith(".json")).sort();
let failed = 0;
for (const name of names) {
  const got = JSON.parse(tg.compare(readFileSync(join(dir, name), "utf8")));
  const want = JSON.parse(readFileSync(join(root, "fixtures/expected", name), "utf8"));
  // The page's packed form must carry exactly the same cells.
  const packed = tg.compare_packed(readFileSync(join(dir, name), "utf8"));
  const head = JSON.parse(packed.json);
  const flat = packed.cells;
  const keys = ["pnl", "pct", "value", "fees", "funding", "delta", "gamma", "theta", "vega"];
  let k = 0;
  for (const c of head.columns) {
    if (c.error) continue;
    c.cells = head.prices.map(() => head.dates.map(() => {
      const cell = Object.fromEntries(keys.map((key, j) => [key, flat[k + j]]));
      cell.liquidated = flat[k + 9] === 1;
      k += 10;
      return cell;
    }));
  }
  const d = diff(got, want, name) ?? diff(head, got, `${name} (packed)`) ?? (k === flat.length ? null : `${name}: packed length`);
  if (d) {
    failed++;
    console.error(`FAIL ${d}`);
  } else {
    console.log(`ok   ${name}`);
  }
}

// The other exports answer and fail the same way as the core.
const check = (cond, what) => {
  if (!cond) {
    failed++;
    console.error(`FAIL ${what}`);
  }
};
const parsed = JSON.parse(tg.parse("BTC to 120k by Dec 31", "2026-11-24"));
check(parsed.target === 120000 && parsed.target_date === "2026-12-31", "parse");
check(JSON.parse(tg.presets()).perps.length === 12, "presets");
check(JSON.parse(tg.strategy_legs("iron_condor", [1, 2, 3, 4])).length === 4, "strategy_legs");
const sc = readFileSync(join(dir, names[0]), "utf8");
const link = tg.to_url(sc, undefined);
check(link.startsWith("https://hunterm777.github.io/ThesisGrid/#s="), "to_url");
check(tg.to_url(tg.from_url(link), undefined) === link, "from_url");
check(tg.csv(sc).startsWith("instrument,price,date"), "csv");
let threw = false;
try {
  tg.compare("{");
} catch (e) {
  threw = typeof e === "string";
}
check(threw, "errors are thrown as strings");

if (failed || names.length < 7) {
  console.error(`${failed} failure(s)`);
  process.exit(1);
}
console.log(`WASM parity: ${names.length} scenarios match to 1e-9`);
