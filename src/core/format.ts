import type { NumberFormat } from "./types";

export function compact(v: number): string {
  const a = Math.abs(v);
  const sign = v < 0 ? "-" : "";
  if (a < 1000) return sign + String(Math.round(a));
  const [n, u] = a < 1e6 ? [a / 1e3, "K"] : a < 1e9 ? [a / 1e6, "M"] : [a / 1e9, "B"];
  if (n < 10) {
    const t = (Math.floor(n * 10) / 10).toFixed(1).replace(/\.0$/, "");
    return sign + t + u;
  }
  return sign + Math.floor(n) + u;
}

function grouped(v: number, decimals: number): string {
  const fixed = Math.abs(v).toFixed(decimals);
  const [int, frac] = fixed.split(".");
  const withSep = int.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  return (v < 0 && Number(fixed) !== 0 ? "-" : "") + withSep + (frac ? "." + frac : "");
}

export function formatValue(v: number | null | undefined, fmt?: Partial<NumberFormat>): string {
  if (v === null || v === undefined || !Number.isFinite(v)) return "—";
  const f: NumberFormat = { style: "full", decimals: null, prefix: "", suffix: "", ...fmt };
  let body: string;
  if (f.style === "compact") {
    body = compact(v);
  } else {
    const decimals = f.decimals ?? (Number.isInteger(v) ? 0 : Math.abs(v) < 10 ? 2 : Math.abs(v) < 1000 ? 2 : 0);
    body = grouped(v, Math.min(Math.max(decimals, 0), 6));
  }
  return f.prefix + body + f.suffix;
}

export function formatDelta(d: number | null | undefined, fmt?: Partial<NumberFormat>): string {
  if (d === null || d === undefined || !Number.isFinite(d) || d === 0) return "";
  const body = formatValue(Math.abs(d), { ...fmt, prefix: "", suffix: "", style: Math.abs(d) >= 10000 ? "compact" : "full" });
  return (d > 0 ? "+" : "−") + body;
}

/** Short forecast date such as "Nov 12", with the year when it is not within the next ~11 months. */
export function etaDate(ts: number, now = Date.now()): string {
  const d = new Date(ts);
  const far = ts - now > 330 * 86400e3;
  return d.toLocaleDateString("en-US", far ? { month: "short", year: "numeric" } : { month: "short", day: "numeric" });
}

/** "153 to go · ~Nov 12" for a numeric goal not reached yet, "" otherwise. */
export function goalLeft(v: { value: number | null; goalEta?: number | null; entry: { goalValue: number | null; format: NumberFormat; textMode?: boolean } }, now = Date.now()): string {
  const g = v.entry.goalValue;
  if (v.entry.textMode || g === null || v.value === null || v.value >= g) return "";
  const left = g - v.value;
  const body = formatValue(left, { ...v.entry.format, style: left >= 10000 ? "compact" : v.entry.format.style });
  return body + " to go" + (v.goalEta ? " · ~" + etaDate(v.goalEta, now) : "");
}

export function slugify(name: string): string {
  let s = name
    .toLowerCase()
    .normalize("NFKD")
    .replace(/[̀-ͯ]/g, "")
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "")
    .slice(0, 40);
  if (!/^[a-z]/.test(s)) s = ("n_" + s).slice(0, 40);
  return s.replace(/_+$/g, "") || "entry";
}

export function timeAgo(ts: number | null, now = Date.now()): string {
  if (!ts) return "never";
  const s = Math.max(0, Math.round((now - ts) / 1000));
  if (s < 10) return "just now";
  if (s < 60) return `${s}s ago`;
  const m = Math.round(s / 60);
  if (m < 60) return `${m} min ago`;
  const h = Math.round(m / 60);
  if (h < 24) return `${h} h ago`;
  return `${Math.round(h / 24)} d ago`;
}

export function intervalLabel(s: number): string {
  if (s < 60) return `${s} s`;
  if (s < 3600) return `${Math.round(s / 60)} min`;
  return `${Math.round(s / 3600)} h`;
}
