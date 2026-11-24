// FR6: P&L at the target date across the grid's price range, one line per
// instrument, on the grids' price axis. Drawn on a canvas so it exports as PNG.
import { download, h } from "../dom";
import { money, price } from "../format";
import { store } from "../state";

export const SERIES = ["#60A5FA", "#F59E0B", "#A78BFA", "#2DD4BF", "#F472B6", "#FB923C", "#E879F9", "#94A3B8"];
const C = { bg: "#13161B", grid: "#22262E", text: "#8B929C", zero: "#4B5260", accent: "#3B82F6" };

const canvas = h("canvas", { role: "img", "aria-label": "Payoff chart at the target date" });
const legend = h("ul", { class: "legend" });
const root = h("section", { class: "chart", "aria-label": "Payoff chart" });
/** Read only on resize, so a redraw never forces a layout of the grids. */
let width = 0;
let legendKey = "";

function ticks(lo: number, hi: number, n: number): number[] {
  const span = hi - lo || 1;
  const raw = span / n;
  const mag = 10 ** Math.floor(Math.log10(raw));
  const step = ([1, 2, 2.5, 5, 10].find((m) => m * mag >= raw) ?? 10) * mag;
  const out: number[] = [];
  for (let v = Math.ceil(lo / step) * step; v <= hi + step * 1e-9; v += step) out.push(v);
  return out;
}

function draw(): void {
  const r = store.result;
  if (!r) return;
  const dpr = window.devicePixelRatio || 1;
  const w = Math.max(320, width || 800);
  const hgt = w < 600 ? 260 : 340;
  if (canvas.width !== Math.round(w * dpr) || canvas.height !== Math.round(hgt * dpr)) {
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(hgt * dpr);
    canvas.style.width = `${w}px`;
    canvas.style.height = `${hgt}px`;
  }
  const g = canvas.getContext("2d");
  if (!g) return;
  g.setTransform(dpr, 0, 0, dpr, 0, 0);
  g.fillStyle = C.bg;
  g.fillRect(0, 0, w, hgt);
  const col = r.target_col;
  const series = r.columns
    .map((c, i) => ({ c, color: SERIES[i % SERIES.length] ?? "#fff", ys: c.cells.map((row) => row[col]?.pnl ?? 0) }))
    .filter((s) => !s.c.error && s.ys.length);
  const xs = r.prices;
  const all = series.flatMap((s) => s.ys);
  let ylo = Math.min(0, ...all);
  let yhi = Math.max(0, ...all);
  if (ylo === yhi) yhi = ylo + 1;
  const pad = (yhi - ylo) * 0.06;
  ylo -= pad;
  yhi += pad;
  const m = { l: 72, r: 16, t: 16, b: 34 };
  const xlo = xs[0] ?? 0;
  const xhi = xs[xs.length - 1] ?? 1;
  const X = (x: number): number => m.l + ((x - xlo) / (xhi - xlo || 1)) * (w - m.l - m.r);
  const Y = (y: number): number => m.t + (1 - (y - ylo) / (yhi - ylo)) * (hgt - m.t - m.b);
  g.font = "11px 'JetBrains Mono', ui-monospace, monospace";
  g.lineWidth = 1;
  g.fillStyle = C.text;
  g.textAlign = "right";
  g.textBaseline = "middle";
  for (const v of ticks(ylo, yhi, 6)) {
    g.strokeStyle = C.grid;
    g.beginPath();
    g.moveTo(m.l, Y(v));
    g.lineTo(w - m.r, Y(v));
    g.stroke();
    g.fillText(money(v), m.l - 8, Y(v));
  }
  g.textAlign = "center";
  g.textBaseline = "top";
  for (const v of ticks(xlo, xhi, w < 600 ? 4 : 8)) g.fillText(price(v), X(v), hgt - m.b + 10);
  g.strokeStyle = C.zero;
  g.beginPath();
  g.moveTo(m.l, Y(0));
  g.lineTo(w - m.r, Y(0));
  g.stroke();
  const target = xs[r.target_row];
  if (target !== undefined) {
    g.strokeStyle = C.accent;
    g.setLineDash([4, 4]);
    g.beginPath();
    g.moveTo(X(target), m.t);
    g.lineTo(X(target), hgt - m.b);
    g.stroke();
    g.setLineDash([]);
  }
  g.lineWidth = 2;
  for (const s of series) {
    g.strokeStyle = s.color;
    g.beginPath();
    s.ys.forEach((y, i) => (i ? g.lineTo(X(xs[i] ?? 0), Y(y)) : g.moveTo(X(xs[i] ?? 0), Y(y))));
    g.stroke();
  }
  const key = series.map((s) => s.c.label + s.color).join("|");
  if (key === legendKey) return;
  legendKey = key;
  legend.replaceChildren(
    ...series.map((s) => {
      const sw = h("span", { class: "swatch", "aria-hidden": "true" });
      sw.style.backgroundColor = s.color;
      return h("li", {}, sw, s.c.label);
    }),
  );
}

export function chartView(): HTMLElement {
  const png = h("button", { type: "button", class: "ghost" }, "Download PNG");
  png.addEventListener("click", () => canvas.toBlob((b) => b && download("thesisgrid-payoff.png", b), "image/png"));
  root.append(
    h("div", { class: "section-head" }, h("h2", {}, "Payoff at the target date"), png),
    canvas,
    legend,
  );
  // The chart redraws once per frame however many edits arrive, and never
  // holds up the grids.
  let queued = false;
  store.subscribe(() => {
    if (queued) return;
    queued = true;
    requestAnimationFrame(() => {
      queued = false;
      draw();
    });
  });
  new ResizeObserver((entries) => {
    const w = Math.round(entries[0]?.contentRect.width ?? 0);
    if (w && w !== width) {
      width = w;
      draw();
    }
  }).observe(root);
  return root;
}
