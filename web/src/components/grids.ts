// The price x date grids: one per instrument, same rows and columns, scrolling
// together; tabs on narrow screens. Cells are reused between redraws so a
// full recompute of four 25 x 30 grids stays well inside one frame.
import { h } from "../dom";
import { cellMoney, money, num, pct, price, shortDate } from "../format";
import { store } from "../state";
import type { Cell, Column, CompareResult } from "../types";

const root = h("section", { class: "grids-wrap", "aria-label": "Grids" });
const tabs = h("div", { class: "tabs", role: "tablist" });
const grids = h("div", { class: "grids" });
const detail = h("div", { class: "detail", role: "status", "aria-live": "polite" });
const liqNote = h(
  "p",
  { class: "note hidden" },
  "Liquidation depends on the price path. A cell is marked liquidated when every path to it must cross the liquidation price.",
);
root.append(tabs, grids, liqNote, detail);

interface Table {
  key: string;
  el: HTMLElement;
  scroller: HTMLElement;
  cells: HTMLTableCellElement[][];
  /** Each cell's text node, rewritten in place. */
  texts: Text[][];
  /** Last text and class written per cell, so a redraw only touches changes. */
  last: string[][];
  /** Price labels and date headers, rewritten in place when the axes move. */
  priceText: Text[];
  dateText: Text[];
  dateTh: HTMLTableCellElement[];
  /** Each row's two <tr> (price table and cells table), for the target marks. */
  rows: HTMLTableRowElement[][];
  marks: [number, number];
}

/** Tables are rebuilt only when their shape changes, not when the axes move. */
function shapeKey(c: Column, r: CompareResult): string {
  return `${c.label}|${r.prices.length}|${r.dates.length}|${c.error ?? ""}`;
}

let tables: Table[] = [];
let active = 0;
let sel: [number, number] | null = null;
let syncing = false;
let tabKey = "";
/** Fixed column width (px) lets the browser lay the tables out in one pass. */
const CELL_W = 76;

function title(c: Column): HTMLElement {
  const venue = c.venue && c.venue !== "custom" ? ` · ${c.venue[0]?.toUpperCase()}${c.venue.slice(1)}` : "";
  // The label is the kind unless renamed; say the kind only when it adds something.
  const kind = c.label === c.kind ? venue.slice(3) : `${c.kind}${venue}`;
  return h("h3", { class: "grid-title" }, c.label, h("span", { class: "muted" }, kind));
}

function build(c: Column, r: CompareResult, i: number): Table {
  const key = shapeKey(c, r);
  const box = h("article", { class: "grid", "data-col": String(i) }, title(c));
  const empty = { priceText: [], dateText: [], dateTh: [], rows: [], marks: [-1, -1] as [number, number] };
  if (c.error) {
    box.append(h("div", { class: "grid-error" }, c.error));
    return { key, el: box, scroller: box, cells: [], texts: [], last: [], ...empty };
  }
  // Prices sit in their own table beside the scrolling cells, so nothing is
  // sticky and the browser can lay out and paint the cells cheaply.
  const priceBody = h("tbody");
  const head = h("tr");
  const dateText: Text[] = [];
  const dateTh: HTMLTableCellElement[] = [];
  r.dates.forEach(() => {
    const text = document.createTextNode("");
    const th = h("th", { scope: "col" }, text);
    head.append(th);
    dateText.push(text);
    dateTh.push(th);
  });
  const body = h("tbody");
  const cells: HTMLTableCellElement[][] = [];
  const texts: Text[][] = [];
  const last: string[][] = [];
  const priceText: Text[] = [];
  const rows: HTMLTableRowElement[][] = [];
  for (let row = r.prices.length - 1; row >= 0; row--) {
    const tr = h("tr");
    const ptext = document.createTextNode("");
    const ptr = h("tr", {}, h("th", { scope: "row" }, ptext));
    priceBody.append(ptr);
    priceText[row] = ptext;
    rows[row] = [ptr, tr];
    const line: HTMLTableCellElement[] = [];
    const tline: Text[] = [];
    r.dates.forEach((_, j) => {
      const text = document.createTextNode("");
      // The fixed-size, contained box stops a text change from re-laying out
      // the whole table: the browser lays out just that one cell.
      const td = h("td", { "data-r": String(row), "data-c": String(j) }, h("div", { class: "v" }, text));
      tr.append(td);
      line.push(td);
      tline.push(text);
    });
    cells[row] = line;
    texts[row] = tline;
    last[row] = r.dates.map(() => "");
    body.append(tr);
  }
  const cols = h("colgroup", {}, ...r.dates.map(() => h("col", { class: "c-cell" })));
  const table = h("table", { class: "cells", "aria-label": `${c.label} P&L by price and date` }, cols, h("thead", {}, head), body);
  table.style.width = `${CELL_W * r.dates.length + 2}px`;
  const prices = h("table", { class: "prices", "aria-hidden": "true" }, h("thead", {}, h("tr", {}, h("th", {}, "Price"))), priceBody);
  const scroller = h("div", { class: "grid-scroll", tabindex: "0", "aria-label": `${c.label} grid; arrow keys move the selection` }, table);
  scroller.addEventListener("scroll", () => {
    if (syncing) return;
    syncing = true;
    for (const t of tables) {
      if (t.scroller !== scroller && t.cells.length) {
        t.scroller.scrollLeft = scroller.scrollLeft;
      }
    }
    syncing = false;
  });
  const pick = (e: Event): void => {
    const td = (e.target as HTMLElement).closest("td");
    if (!td) return;
    select(Number(td.dataset["r"]), Number(td.dataset["c"]));
  };
  table.addEventListener("pointerover", pick);
  table.addEventListener("click", pick);
  scroller.addEventListener("keydown", (e) => {
    const res = store.result;
    if (!res) return;
    const [r0, c0] = sel ?? [res.target_row, res.target_col];
    const moves: Record<string, [number, number]> = { ArrowUp: [1, 0], ArrowDown: [-1, 0], ArrowLeft: [0, -1], ArrowRight: [0, 1] };
    const m = moves[e.key];
    if (!m) return;
    e.preventDefault();
    const rr = Math.min(res.prices.length - 1, Math.max(0, r0 + m[0]));
    const cc = Math.min(res.dates.length - 1, Math.max(0, c0 + m[1]));
    select(rr, cc);
    tables[i]?.cells[rr]?.[cc]?.scrollIntoView({ block: "nearest", inline: "nearest" });
  });
  box.append(h("div", { class: "grid-body" }, prices, scroller));
  return { key, el: box, scroller, cells, texts, last, priceText, dateText, dateTh, rows, marks: [-1, -1] };
}

/** Writes the axis labels and moves the target row and column marks. */
function axes(t: Table, r: CompareResult): void {
  r.prices.forEach((p, row) => {
    const node = t.priceText[row];
    const text = price(p);
    if (node && node.data !== text) node.data = text;
  });
  r.dates.forEach((d, j) => {
    const node = t.dateText[j];
    const text = shortDate(d);
    if (node && node.data !== text) node.data = text;
  });
  const [tr0, tc0] = t.marks;
  if (tr0 !== r.target_row) {
    for (const el of t.rows[tr0] ?? []) el.classList.remove("target-row");
    for (const el of t.rows[tr0 + 1] ?? []) el.classList.remove("pre-target");
    for (const el of t.rows[r.target_row] ?? []) el.classList.add("target-row");
    for (const el of t.rows[r.target_row + 1] ?? []) el.classList.add("pre-target");
  }
  if (tc0 !== r.target_col) {
    t.dateTh[tc0]?.classList.remove("target-col");
    t.dateTh[r.target_col]?.classList.add("target-col");
    // Cell classes carry the column mark; force those cells to be rewritten.
    for (const line of t.last) {
      if (tc0 >= 0) line[tc0] = "";
      line[r.target_col] = "";
    }
  }
  t.marks = [r.target_row, r.target_col];
}

/** Background shade by magnitude: 16 steps per sign keeps redraws cheap. */
function shade(x: number, max: number): string {
  if (x === 0 || max <= 0) return "";
  const step = Math.round(15 * Math.sqrt(Math.min(1, Math.abs(x) / max)));
  return `${x > 0 ? "p" : "n"}${step}`;
}

function fill(t: Table, c: Column, percent: boolean, max: number, targetCol: number): void {
  for (let i = 0; i < c.cells.length; i++) {
    const row = c.cells[i] ?? [];
    const tds = t.cells[i] ?? [];
    const texts = t.texts[i] ?? [];
    const last = t.last[i] ?? [];
    for (let j = 0; j < row.length; j++) {
      const cell = row[j];
      const td = tds[j];
      const node = texts[j];
      if (!cell || !td || !node) continue;
      const v = percent ? cell.pct : cell.pnl;
      const text = cell.liquidated ? "liq" : percent ? pct(v, 0) : cellMoney(v);
      const cls = cell.liquidated ? "liq" : v < 0 ? `neg ${shade(v, max)}` : shade(v, max);
      const state = text + cls;
      if (last[j] === state) continue;
      if (node.data !== text) node.data = text;
      const sel1 = sel !== null && sel[0] === i && sel[1] === j;
      td.className = `${j === targetCol ? "target-col " : ""}${cls}${sel1 ? " sel" : ""}`;
      last[j] = state;
    }
  }
}

function line(label: string, c: Cell): HTMLElement {
  const parts = [
    `P&L ${money(c.pnl, true)} (${pct(c.pct)})`,
    `value ${money(c.value)}`,
    `fees ${money(c.fees)}`,
    `funding ${money(c.funding)}`,
    `delta ${num(c.delta)}`,
    `gamma ${num(c.gamma)}`,
    `theta/day ${num(c.theta)}`,
    `vega/pt ${num(c.vega)}`,
  ];
  return h("li", {}, h("strong", {}, label), c.liquidated ? " liquidated · " : " ", parts.join(" · "));
}

function select(row: number, col: number): void {
  const r = store.result;
  if (!r) return;
  if (sel) for (const t of tables) t.cells[sel[0]]?.[sel[1]]?.classList.remove("sel");
  sel = [row, col];
  for (const t of tables) t.cells[row]?.[col]?.classList.add("sel");
  const items = r.columns.filter((c) => !c.error).map((c) => {
    const cell = c.cells[row]?.[col];
    return cell ? line(c.label, cell) : null;
  });
  detail.replaceChildren(
    h("p", { class: "detail-head" }, `At ${price(r.prices[row] ?? 0)} on ${shortDate(r.dates[col] ?? "")}`),
    h("ul", {}, ...items),
  );
}

function render(): void {
  const r = store.result;
  if (!r) return;
  const percent = r.display === "percent";
  let max = 0;
  for (const c of r.columns) for (const row of c.cells) for (const cell of row) {
    if (!cell.liquidated) max = Math.max(max, Math.abs(percent ? cell.pct : cell.pnl));
  }
  const next: Table[] = r.columns.map((c, i) => {
    const old = tables[i];
    return old && old.key === shapeKey(c, r) ? old : build(c, r, i);
  });
  if (next.some((t, i) => t !== tables[i]) || next.length !== tables.length) {
    tables = next;
    grids.replaceChildren(...tables.map((t) => t.el));
  }
  r.columns.forEach((c, i) => {
    const t = tables[i];
    if (t && !c.error) {
      axes(t, r);
      fill(t, c, percent, max, r.target_col);
    }
  });
  active = Math.min(active, Math.max(0, r.columns.length - 1));
  const labels = r.columns.map((c) => c.label).join("\u0000");
  if (labels !== tabKey) {
    tabKey = labels;
    tabs.replaceChildren(
      ...r.columns.map((c, i) => {
        const b = h("button", { type: "button", role: "tab" }, c.label);
        b.addEventListener("click", () => {
          active = i;
          render();
        });
        return b;
      }),
    );
  }
  [...tabs.children].forEach((b, i) => b.setAttribute("aria-selected", String(i === active)));
  tables.forEach((t, i) => t.el.classList.toggle("active", i === active));
  liqNote.classList.toggle("hidden", !r.columns.some((c) => c.kind === "perp" && !c.error));
  const [sr, sc] = sel ?? [r.target_row, r.target_col];
  select(Math.min(sr, r.prices.length - 1), Math.min(sc, r.dates.length - 1));
}

export function gridView(): HTMLElement {
  store.subscribe(() => render());
  return root;
}
