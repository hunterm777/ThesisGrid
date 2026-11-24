// Comparison mode toggle with the core's one-line sizing note, the error
// banner, and the grid's layout controls.
import { h, numField, segmented } from "../dom";
import { store } from "../state";
import type { Display, Mode, PriceAxis } from "../types";

const MODES = [
  ["equal_dollars", "Equal dollars"],
  ["equal_max_loss", "Equal max loss"],
  ["equal_exposure", "Equal exposure"],
  ["manual", "Manual"],
] as const;

const root = h("section", { class: "mode", "aria-label": "Comparison basis" });
const note = h("p", { class: "sizing-note", role: "status", "aria-live": "polite" });
const banner = h("p", { class: "banner hidden", role: "alert" });

function renderToggle(): void {
  const mode = store.sc.mode ?? "equal_dollars";
  const toggle = segmented<Mode>("Comparison basis", mode, MODES, (v) => store.update((sc) => (sc.mode = v), true));
  root.replaceChildren(toggle, note, banner);
}

function update(): void {
  note.textContent = store.result?.sizing_note ?? "";
  const msg = store.error ?? store.linkError;
  banner.textContent = msg ?? "";
  banner.classList.toggle("hidden", !msg);
}

export function modeToggle(): HTMLElement {
  store.subscribe((structural) => {
    if (structural) renderToggle();
    update();
  });
  return root;
}

/** Rows, columns and the dollars/percent toggle, above the grids. */
export function gridControls(): HTMLElement {
  const el = h("div", { class: "grid-controls" });
  const render = (): void => {
    const g = store.sc.grid ?? {};
    const p: PriceAxis = g.prices ?? { kind: "range_pct", lo: -0.3, hi: 0.3, steps: 25 };
    const pctAxis = p.kind === "range_pct";
    const setAxis = (key: "lo" | "hi" | "steps") => (v: number | undefined) =>
      store.update((sc) => {
        const grid = (sc.grid ??= {});
        const axis: PriceAxis = { kind: "range_pct", lo: -0.3, hi: 0.3, steps: 25, ...(pctAxis ? p : {}) };
        if (v !== undefined) axis[key] = key === "steps" ? Math.round(v) : v;
        grid.prices = axis;
      });
    const display = segmented<Display>("Cell values", g.display ?? "dollars", [["dollars", "$"], ["percent", "% of capital"]] as const, (v) =>
      store.update((sc) => ((sc.grid ??= {}).display = v), true),
    );
    el.replaceChildren(
      numField({ label: "Rows from", value: pctAxis ? (p.lo ?? -0.3) : -0.3, scale: 100, suffix: "%", onValue: setAxis("lo") }),
      numField({ label: "to", value: pctAxis ? (p.hi ?? 0.3) : 0.3, scale: 100, suffix: "%", onValue: setAxis("hi") }),
      numField({ label: "Row steps", value: pctAxis ? (p.steps ?? 25) : 25, onValue: setAxis("steps") }),
      numField({
        label: "Days per column",
        value: g.dates?.step,
        placeholder: "auto",
        onValue: (v) =>
          store.update((sc) => {
            const d = ((sc.grid ??= {}).dates ??= {});
            if (v === undefined) delete d.step;
            else d.step = Math.max(1, Math.round(v));
          }),
      }),
      display,
    );
  };
  store.subscribe((structural) => {
    if (structural) render();
  });
  return el;
}
