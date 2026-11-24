// Display formatting only; every value shown here was computed by the core.

const whole = new Intl.NumberFormat("en-US", { maximumFractionDigits: 0 });
const cents = new Intl.NumberFormat("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
const sig4 = new Intl.NumberFormat("en-US", { maximumSignificantDigits: 4 });

/** "$1,234" or "$12.34"; a minus sign for negatives, never parentheses. */
export function money(x: number | null | undefined, signed = false): string {
  if (x === null || x === undefined) return "Unbounded";
  if (!Number.isFinite(x)) return "–";
  const a = Math.abs(x);
  const body = a >= 1000 ? whole.format(a) : cents.format(a);
  const zero = body === "0.00" || body === "0";
  const sign = x < 0 && !zero ? "-" : signed && x > 0 && !zero ? "+" : "";
  return `${sign}$${body}`;
}

/** Thousands separators for a whole number (hand-rolled: grids format ~3,000
 * cells per redraw, and Intl is several times slower). */
function commas(n: number): string {
  const s = String(n);
  if (s.length <= 3) return s;
  let out = "";
  for (let i = 0; i < s.length; i++) {
    if (i > 0 && (s.length - i) % 3 === 0) out += ",";
    out += s[i];
  }
  return out;
}

/** Compact cell text: whole dollars from $100, cents below, sign always shown. */
export function cellMoney(x: number): string {
  const a = Math.abs(x);
  let body: string;
  if (Math.round(a) >= 100) body = commas(Math.round(a));
  else {
    const c = Math.round(a * 100);
    if (c === 0) return "0";
    body = c >= 10000 ? commas(Math.round(a)) : `${Math.floor(c / 100)}.${String(c % 100).padStart(2, "0")}`;
  }
  return `${x < 0 ? "-" : "+"}${body}`;
}

/** "+12.3%" / "-4.0%". */
export function pct(x: number, dp = 1): string {
  if (!Number.isFinite(x)) return "–";
  const s = Math.abs(x).toFixed(dp);
  if (Number(s) === 0) return "0%";
  return `${x < 0 ? "-" : "+"}${s}%`;
}

/** A price: "$95,000", "$182.40", "$0.31". */
export function price(x: number): string {
  const a = Math.abs(x);
  const f = a >= 1000 ? whole : a >= 1 ? cents : sig4;
  return `$${f.format(x)}`;
}

/** A plain number to `sig` significant digits. */
export function num(x: number, sig = 4): string {
  if (!Number.isFinite(x)) return "–";
  if (x === 0) return "0";
  return Number(x.toPrecision(sig)).toLocaleString("en-US", { maximumFractionDigits: 10 });
}

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** "Dec 31" from "2026-12-31". */
export function shortDate(iso: string): string {
  const m = Number(iso.slice(5, 7));
  return `${MONTHS[m - 1] ?? "?"} ${Number(iso.slice(8, 10))}`;
}

/** Today in the viewer's own time zone, as YYYY-MM-DD. */
export function today(): string {
  const d = new Date();
  const p = (n: number): string => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

/** Calendar date arithmetic for default dates only (no pricing). */
export function addDays(iso: string, days: number): string {
  const [y, m, d] = iso.split("-").map(Number) as [number, number, number];
  const t = new Date(Date.UTC(y, m - 1, d + days));
  return t.toISOString().slice(0, 10);
}
