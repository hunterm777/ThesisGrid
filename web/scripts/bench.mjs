// Loads the built page in headless Chrome and checks the PRD's page rules:
// no requests after load, no CSP violations or console errors, and a full
// recompute + redraw of four 25 x 30 grids under 16 ms (the median of 120
// edits is the gate; p95 and max are reported alongside it).
// Usage: CHROME=/path/to/chrome node scripts/bench.mjs   (needs puppeteer-core)
import { createServer } from "node:http";
import { readFileSync, existsSync } from "node:fs";
import { extname, join, resolve } from "node:path";

const { default: puppeteer } = await import("puppeteer-core");
const dist = resolve(import.meta.dirname, "../dist");
const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".woff2": "font/woff2", ".woff": "font/woff" };
const server = createServer((req, res) => {
  const path = join(dist, decodeURIComponent((req.url ?? "/").split("?")[0]).replace(/\/$/, "/index.html"));
  if (!path.startsWith(dist) || !existsSync(path)) return res.writeHead(404).end();
  res.writeHead(200, { "content-type": types[extname(path)] ?? "application/octet-stream" }).end(readFileSync(path));
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const url = `http://127.0.0.1:${server.address().port}/`;

// The PRD's benchmark: four instruments on 25 x 30 grids.
const bench = {
  thesis: { underlying: "BTC", asset_class: "crypto", spot: 100000, target: 115000, target_date: "2026-12-20",
    as_of: "2026-11-24", rate: 0.04, iv: 0.55 },
  mode: "equal_max_loss", budget: 500,
  instruments: [
    { kind: "spot", stop: 90000 },
    { kind: "perp", venue: "hyperliquid", leverage: 5, funding_rate: 0.0001 },
    { kind: "option", strategy: "bull_call_spread", strikes: [105000, 120000], expiry: "2026-12-20" },
    { kind: "binary", venue: "kalshi", strike: 115000, expiry: "2026-12-20", price: 0.2 },
  ],
  grid: { prices: { kind: "range_pct", lo: -0.3, hi: 0.3, steps: 24 }, dates: { step: 1, end: "2026-12-23" } },
};
const benchUrl = `${url}#s=${Buffer.from(JSON.stringify(bench)).toString("base64url")}`;

const args = process.env.CHROME_ARGS ? JSON.parse(process.env.CHROME_ARGS) : ["--no-sandbox"];
const browser = await puppeteer.launch({ executablePath: process.env.CHROME, args, headless: true });
const page = await browser.newPage();
await page.setViewport({ width: 1440, height: 1000 });
const problems = [];
const requests = [];
let loaded = false;
page.on("console", (m) => m.type() === "error" && problems.push(`console: ${m.text()}`));
page.on("pageerror", (e) => problems.push(`page error: ${e.message}`));
page.on("request", (r) => loaded && requests.push(r.url()));
await page.evaluateOnNewDocument(() => {
  document.addEventListener("securitypolicyviolation", (e) => console.error(`CSP ${e.violatedDirective} ${e.blockedURI}`));
});
await page.goto(benchUrl, { waitUntil: "networkidle0" });
loaded = true;

const stats = await page.evaluate(async () => {
  const spot = [...document.querySelectorAll("label.field")].find((l) => l.textContent?.startsWith("Current price"))?.querySelector("input");
  const tables = document.querySelectorAll(".grid table.cells");
  const grids = tables.length;
  const rows = tables[0]?.querySelectorAll("tbody tr").length ?? 0;
  const cols = tables[0]?.querySelectorAll("thead th").length ?? 0;
  // Each edit is timed from the input event through the core, the redraw and
  // the browser's style and layout pass that follows it.
  const times = [];
  for (let i = 0; i < 120; i++) {
    spot.value = String(95000 + i * 100);
    const t0 = performance.now();
    spot.dispatchEvent(new Event("input", { bubbles: true }));
    void document.body.offsetHeight;
    times.push(performance.now() - t0);
    await new Promise((r) => requestAnimationFrame(r));
  }
  times.sort((a, b) => a - b);
  const r = (x) => Math.round(x * 10) / 10;
  return { grids, rows, cols, median: r(times[60]), p95: r(times[114]), max: r(times[119]), note: document.querySelector(".sizing-note")?.textContent };
});
await page.screenshot({ path: process.env.SHOT ?? "/tmp/thesisgrid.png", fullPage: true });

// Quick entry fills the thesis and leaves the current price to the user.
const quick = await page.evaluate(() => {
  const q = document.querySelector(".quick-input");
  q.value = "NVDA to $200 by Mar 20";
  q.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
  const val = (name) => [...document.querySelectorAll("label.field")].find((l) => l.textContent?.startsWith(name))?.querySelector("input,select")?.value;
  return { underlying: val("Underlying"), asset: val("Asset class"), target: val("Target price"), msg: document.querySelector(".quick-msg")?.textContent };
});

// A shared link reproduces the same page (the address follows edits once
// typing pauses).
await new Promise((ok) => setTimeout(ok, 600));
const link = page.url();
const page2 = await browser.newPage();
await page2.goto(link, { waitUntil: "networkidle0" });
const note2 = await page2.evaluate(() => document.querySelector(".sizing-note")?.textContent);
const note1 = await page.evaluate(() => document.querySelector(".sizing-note")?.textContent);
await page2.setViewport({ width: 390, height: 844, isMobile: true });
await page2.screenshot({ path: process.env.SHOT_MOBILE ?? "/tmp/thesisgrid-mobile.png", fullPage: true });

await browser.close();
server.close();
console.log(JSON.stringify({ stats, quick, linkMatches: note1 === note2, requestsAfterLoad: requests, problems }, null, 1));
const fail = problems.length || requests.length || stats.grids < 4 || stats.median >= 16 || note1 !== note2;
process.exit(fail ? 1 : 0);
