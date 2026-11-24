// Tiny DOM helpers. No styles are set through attributes, so the page runs
// under a CSP without 'unsafe-inline'.

type Attrs = Record<string, string | number | boolean | undefined | EventListener>;
type Child = Node | string | null | undefined | false;

export function h<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs: Attrs = {},
  ...children: (Child | Child[])[]
): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (v === undefined || v === false) continue;
    if (typeof v === "function") el.addEventListener(k.replace(/^on/, ""), v);
    else if (k === "class") el.className = String(v);
    else if (v === true) el.setAttribute(k, "");
    else el.setAttribute(k, String(v));
  }
  for (const c of children.flat()) {
    if (c === null || c === undefined || c === false) continue;
    el.append(typeof c === "string" ? document.createTextNode(c) : c);
  }
  return el;
}

let uid = 0;
export const nextId = (p: string): string => `${p}-${++uid}`;

/** Trims float noise for showing a stored number back in an input. */
export function show(x: number | undefined, scale = 1): string {
  if (x === undefined || !Number.isFinite(x)) return "";
  return String(Number((x * scale).toPrecision(12)));
}

const SUFFIX: Record<string, number> = { k: 1e3, m: 1e6, b: 1e9 };

/** Reads an input; "" is undefined, junk is NaN. Accepts "120k", "3,400". */
export function read(v: string, scale = 1): number | undefined {
  const t = v.trim().replace(/[,$%\s]/g, "");
  if (t === "") return undefined;
  const last = t.slice(-1).toLowerCase();
  const mult = /^-?[\d.]+[kmb]$/i.test(t) ? (SUFFIX[last] ?? 1) : 1;
  const n = Number(mult === 1 ? t : t.slice(0, -1)) * mult;
  return Number.isFinite(n) ? Number((n / scale).toPrecision(15)) : Number.NaN;
}

export interface NumOpts {
  label: string;
  value: number | undefined;
  scale?: number;
  placeholder?: string;
  suffix?: string;
  required?: boolean;
  onValue: (v: number | undefined) => void;
}

export function numField(o: NumOpts): HTMLLabelElement {
  const id = nextId("f");
  const input = h("input", {
    id,
    type: "text",
    inputmode: "decimal",
    autocomplete: "off",
    spellcheck: "false",
    class: "num",
    value: show(o.value, o.scale),
    placeholder: o.placeholder,
  });
  input.addEventListener("input", () => {
    const v = read(input.value, o.scale);
    const bad = Number.isNaN(v) || (o.required === true && v === undefined);
    input.classList.toggle("invalid", bad);
    input.setAttribute("aria-invalid", String(bad));
    if (!bad) o.onValue(v);
  });
  return h(
    "label",
    { class: "field", for: id },
    h("span", { class: "flabel" }, o.label),
    h("span", { class: "fbox" }, input, o.suffix ? h("span", { class: "suffix" }, o.suffix) : null),
  );
}

export function selectField<T extends string>(
  label: string,
  value: T,
  options: readonly (readonly [T, string])[],
  onValue: (v: T) => void,
): HTMLLabelElement {
  const id = nextId("f");
  const sel = h("select", { id }, ...options.map(([v, t]) => h("option", { value: v, selected: v === value }, t)));
  sel.addEventListener("change", () => onValue(sel.value as T));
  return h("label", { class: "field", for: id }, h("span", { class: "flabel" }, label), sel);
}

export function dateField(
  label: string,
  value: string | undefined,
  onValue: (v: string | undefined) => void,
): HTMLLabelElement {
  const id = nextId("f");
  const input = h("input", { id, type: "date", value: value ?? "" });
  input.addEventListener("change", () => onValue(input.value || undefined));
  return h("label", { class: "field", for: id }, h("span", { class: "flabel" }, label), input);
}

export function textField(
  label: string,
  value: string,
  onValue: (v: string) => void,
  placeholder = "",
): HTMLLabelElement {
  const id = nextId("f");
  const input = h("input", { id, type: "text", value, placeholder, autocomplete: "off", spellcheck: "false" });
  input.addEventListener("input", () => onValue(input.value));
  return h("label", { class: "field", for: id }, h("span", { class: "flabel" }, label), input);
}

export function segmented<T extends string>(
  name: string,
  value: T,
  options: readonly (readonly [T, string])[],
  onValue: (v: T) => void,
): HTMLDivElement {
  const group = h("div", { class: "segmented", role: "radiogroup", "aria-label": name });
  for (const [v, t] of options) {
    const on = v === value;
    const b = h("button", { type: "button", role: "radio", "aria-checked": String(on), class: on ? "on" : "" }, t);
    b.addEventListener("click", () => onValue(v));
    group.append(b);
  }
  return group;
}

/** Saves a blob through a temporary link; nothing leaves the browser. */
export function download(name: string, blob: Blob): void {
  const url = URL.createObjectURL(blob);
  const a = h("a", { href: url, download: name, class: "hidden" });
  document.body.append(a);
  a.click();
  document.body.removeChild(a);
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
