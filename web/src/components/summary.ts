// FR5: one summary row per instrument, under the grids.
import { h } from "../dom";
import { money, num, pct, price } from "../format";
import { store } from "../state";

const root = h("section", { class: "summary", "aria-label": "Summary" });

function render(): void {
  const r = store.result;
  if (!r) return;
  const head = ["Instrument", "Size", "Cost to enter", "Max loss", "Max gain", "Breakeven at target date", "P&L at target", "Return at target", "Liquidation"];
  const rows = r.columns.map((c) => {
    const s = c.summary;
    const cells = s
      ? [
          num(s.qty, 6),
          money(s.cost),
          money(s.max_loss),
          money(s.max_gain),
          s.breakevens.length ? s.breakevens.map(price).join(", ") : "none",
          money(s.pnl_at_target, true),
          pct(s.roc_at_target),
          s.liquidation === null ? "–" : price(s.liquidation),
        ]
      : [c.error ?? "", "", "", "", "", "", "", ""];
    const tds = cells.map((t, i) => {
      const neg = s && ((i === 5 && s.pnl_at_target < 0) || (i === 6 && s.roc_at_target < 0));
      return h("td", { class: neg ? "neg" : s && (i === 5 || i === 6) ? "pos" : "" }, t);
    });
    const notes = c.notes.length ? h("ul", { class: "col-notes" }, ...c.notes.map((n) => h("li", {}, n))) : null;
    return [
      h("tr", {}, h("th", { scope: "row" }, c.label), ...tds),
      notes ? h("tr", { class: "notes-row" }, h("td", { colspan: String(head.length) }, notes)) : null,
    ];
  });
  root.replaceChildren(
    h("h2", {}, "Summary"),
    h(
      "div",
      { class: "table-scroll", tabindex: "0", "aria-label": "Summary table" },
      h("table", {}, h("thead", {}, h("tr", {}, ...head.map((t) => h("th", { scope: "col" }, t)))), h("tbody", {}, ...rows.flat())),
    ),
  );
}

export function summaryView(): HTMLElement {
  store.subscribe(() => render());
  return root;
}
