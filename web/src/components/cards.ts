// Instrument cards: one per grid column, collapsible, plus a "+" card.
import { h, nextId, numField } from "../dom";
import { store } from "../state";
import type { Spec } from "../types";
import { binaryFields, optionFields, perpFields, spotFields } from "./fields";

const TITLES: Record<Spec["kind"], string> = { spot: "Shares / spot", perp: "Perp", option: "Options", binary: "Binary" };
const collapsed = new WeakSet<Spec>();

function fresh(kind: Spec["kind"]): Spec {
  const th = store.sc.thesis;
  switch (kind) {
    case "spot":
      return { kind, side: "long" };
    case "perp":
      return { kind, venue: "hyperliquid", side: "long", leverage: 5 };
    case "option":
      return { kind, strategy: "long_call", strikes: [th.target], expiry: th.target_date };
    case "binary":
      return { kind, venue: "kalshi", side: "yes", strike: th.target, expiry: th.target_date, price: 0.5 };
  }
}

function card(spec: Spec, index: number, label: string): HTMLElement {
  const body = h("div", { class: "card-body", id: nextId("card") });
  const isOpen = !collapsed.has(spec);
  const toggle = h("button", { type: "button", class: "ghost", "aria-expanded": String(isOpen), "aria-controls": body.id }, isOpen ? "Hide" : "Show");
  toggle.addEventListener("click", () => {
    if (collapsed.has(spec)) collapsed.delete(spec);
    else collapsed.add(spec);
    render();
  });
  const enabled = h("input", { type: "checkbox", checked: spec["enabled"] !== false, "aria-label": `Show ${label}` });
  enabled.addEventListener("change", () =>
    store.update(() => {
      if (enabled.checked) delete spec["enabled"];
      else spec["enabled"] = false;
    }),
  );
  const remove = h("button", { type: "button", class: "ghost danger", "aria-label": `Remove ${label}`, title: "Remove" }, "×");
  remove.addEventListener("click", () => store.update((sc) => sc.instruments.splice(index, 1), true));
  const name = h("input", {
    type: "text",
    class: "card-name",
    value: (spec["label"] as string | undefined) ?? "",
    placeholder: label,
    "aria-label": "Column name",
  });
  name.addEventListener("change", () =>
    store.update(() => {
      if (name.value.trim()) spec["label"] = name.value.trim();
      else delete spec["label"];
    }),
  );
  const lock = numField({
    label: "Size lock",
    value: spec["qty"] as number | undefined,
    placeholder: "by mode",
    onValue: (v) =>
      store.update(() => {
        if (v === undefined) delete spec["qty"];
        else spec["qty"] = v;
      }),
  });
  lock.classList.add("lock");
  if (isOpen) {
    const fields =
      spec.kind === "spot"
        ? spotFields(spec)
        : spec.kind === "perp"
          ? perpFields(spec)
          : spec.kind === "option"
            ? optionFields(spec)
            : binaryFields(spec);
    body.append(h("div", { class: "fields" }, ...fields, lock));
  }
  return h(
    "section",
    { class: spec["enabled"] === false ? "card off" : "card", "aria-label": label },
    h("header", { class: "card-head" }, enabled, h("span", { class: "kind" }, TITLES[spec.kind]), name, toggle, remove),
    body,
  );
}

const root = h("div", { class: "cards" });

/** Column names as the core will number them ("perp", "perp 2"). */
function labels(): string[] {
  const seen: string[] = [];
  return store.sc.instruments.map((s) => {
    const base = (s["label"] as string | undefined) ?? s.kind;
    const n = seen.filter((x) => x === base).length;
    seen.push(base);
    return n === 0 ? base : `${base} ${n + 1}`;
  });
}

function render(): void {
  const names = labels();
  const add = h("div", { class: "card add" }, h("span", { class: "kind" }, "+ Add"));
  for (const kind of ["spot", "perp", "option", "binary"] as const) {
    const b = h("button", { type: "button" }, TITLES[kind]);
    b.addEventListener("click", () => store.update((sc) => sc.instruments.push(fresh(kind)), true));
    add.append(b);
  }
  root.replaceChildren(...store.sc.instruments.map((s, i) => card(s, i, names[i] ?? s.kind)), add);
}

export function cards(): HTMLElement {
  store.subscribe((structural) => {
    if (structural) render();
  });
  return root;
}
