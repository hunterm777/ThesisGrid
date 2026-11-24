// Header, quick entry and the thesis bar.
import * as core from "../core";
import { dateField, h, nextId, numField, selectField, textField } from "../dom";
import { store } from "../state";
import type { AssetClass, Thesis } from "../types";

export function header(): HTMLElement {
  return h(
    "header",
    { class: "top" },
    h("div", { class: "brand" }, h("span", { class: "logo", "aria-hidden": "true" }, "▦"), h("h1", {}, "ThesisGrid")),
    h("p", { class: "tagline" }, "Same thesis, every instrument, one grid."),
    h("a", { class: "gh", href: "https://github.com/hunterm777/ThesisGrid", rel: "noopener" }, "GitHub"),
  );
}

const ASSETS = [
  ["crypto", "Crypto"],
  ["stock", "Stock"],
  ["index", "Index / ETF"],
  ["commodity", "Commodity"],
] as const;

const BUDGET_LABEL = {
  equal_dollars: "Capital",
  equal_max_loss: "Risk",
  equal_exposure: "Delta",
  manual: "Budget",
} as const;

const root = h("section", { class: "thesis", "aria-label": "Thesis" });
const quickMsg = h("p", { class: "quick-msg", role: "status" });
let missing = new Set<string>();
let quickEl: HTMLElement | null = null;

function set<K extends keyof Thesis>(key: K, structural = false) {
  return (v: Thesis[K] | undefined): void =>
    store.update((sc) => {
      missing.delete(key);
      if (v === undefined) delete sc.thesis[key];
      else sc.thesis[key] = v;
    }, structural);
}

function quick(): HTMLElement {
  const id = nextId("q");
  const input = h("input", {
    id,
    type: "text",
    class: "quick-input",
    placeholder: "BTC to 120k by Dec 31  ·  NVDA to $200 by Mar 20  ·  gold below 3,400 by Jan 15",
    autocomplete: "off",
    spellcheck: "false",
  });
  const run = (): void => {
    if (!input.value.trim()) return;
    const p = core.parse(input.value, store.sc.thesis.as_of);
    store.update((sc) => {
      const t = sc.thesis;
      if (p.underlying) t.underlying = p.underlying;
      if (p.asset_class) t.asset_class = p.asset_class;
      if (p.target !== null) t.target = p.target;
      if (p.target_date) t.target_date = p.target_date;
      if (p.touch) t.touch = p.touch;
      if (p.direction) t.direction = p.direction;
      else delete t.direction;
    }, true);
    missing = new Set(p.missing);
    missing.add("spot");
    const unread = p.missing.map((m) => m.replace("_", " "));
    quickMsg.textContent =
      (unread.length ? `Could not read: ${unread.join(", ")}. ` : "") + "Now type the current price.";
    renderBar();
    root.querySelector<HTMLInputElement>(".need-spot input")?.focus();
  };
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") run();
  });
  const go = h("button", { type: "button", class: "primary" }, "Fill");
  go.addEventListener("click", run);
  return h(
    "div",
    { class: "quick" },
    h("label", { for: id, class: "flabel" }, "Quick entry"),
    h("div", { class: "quick-row" }, input, go),
    quickMsg,
  );
}

function renderBar(): void {
  const t = store.sc.thesis;
  const mode = store.sc.mode ?? "equal_dollars";
  const mark = (el: HTMLElement, key: string): HTMLElement => {
    if (missing.has(key)) el.classList.add("missing");
    if (key === "spot") el.classList.add("need-spot");
    return el;
  };
  const fields = [
    mark(textField("Underlying", t.underlying ?? "", (v) => set("underlying")(v.trim() || undefined), "BTC"), "underlying"),
    mark(selectField<AssetClass>("Asset class", t.asset_class ?? "crypto", ASSETS, (v) => set("asset_class", true)(v)), "asset_class"),
    mark(numField({ label: "Current price", value: t.spot, required: true, onValue: (v) => set("spot")(v) }), "spot"),
    mark(numField({ label: "Target price", value: t.target, required: true, onValue: (v) => set("target")(v) }), "target"),
    mark(dateField("Target date", t.target_date, (v) => v && set("target_date")(v)), "target_date"),
    selectField("Reaches it", t.touch ?? "at", [["at", "At the date"], ["by", "By the date"]] as const, (v) => set("touch")(v)),
    numField({
      label: BUDGET_LABEL[mode],
      value: store.sc.budget ?? 1000,
      suffix: "$",
      required: true,
      onValue: (v) => v !== undefined && store.update((sc) => (sc.budget = v)),
    }),
    numField({ label: "Rate", value: t.rate ?? 0, scale: 100, suffix: "%", onValue: (v) => set("rate")(v) }),
    numField({ label: "Implied vol", value: t.iv ?? 0.5, scale: 100, suffix: "%", onValue: (v) => set("iv")(v) }),
    dateField("Today", t.as_of, (v) => v && set("as_of")(v)),
  ];
  if (t.asset_class === "commodity") {
    fields.push(
      selectField("Year basis", t.commodity_day_count ?? "calendar365", [["calendar365", "365 calendar"], ["trading252", "252 trading"]] as const, (v) => set("commodity_day_count")(v)),
    );
  }
  quickEl ??= quick();
  root.replaceChildren(quickEl, h("div", { class: "bar" }, ...fields));
}

export function thesisBar(): HTMLElement {
  store.subscribe((structural) => {
    if (structural) renderBar();
  });
  return root;
}
