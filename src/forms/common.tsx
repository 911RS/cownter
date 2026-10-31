import { formatDelta, formatValue } from "../core/format";
import { SIZE_SCALE } from "../core/select";
import type { EntryView, FormCfg, Status } from "../core/types";

export function valueText(v: EntryView, compactStyle = false): string {
  if (v.entry.textMode) {
    const t = v.text ?? (v.status === "polling" ? "..." : "—");
    const max = compactStyle ? 14 : 28;
    return t.length > max ? t.slice(0, max - 1).trimEnd() + "…" : t;
  }
  const fmt = compactStyle ? { ...v.entry.format, style: "compact" as const } : v.entry.format;
  if (v.value === null) return v.status === "polling" ? "..." : "—";
  return formatValue(v.value, fmt);
}

export function Delta({ v }: { v: EntryView }) {
  const t = formatDelta(v.deltaToday, v.entry.format);
  if (!t) return null;
  return <span className={"delta " + ((v.deltaToday ?? 0) > 0 ? "up" : "down")}>{t}</span>;
}

export function StatusMark({ status, error }: { status: Status; error: string | null }) {
  if (status === "stale") return <span className="smark stale" title={error ?? "Not up to date"}>◷</span>;
  if (status === "error" || status === "noLink") return <span className="smark err" title={error ?? "Error"}>!</span>;
  return null;
}

/** Combined scale of a form: size preset times the fine scale. */
export function formScale(f: FormCfg): number {
  return (SIZE_SCALE[f.size] ?? 1) * (f.scale || 1);
}

function rgba(hex: string, a: number): string {
  const n = /^#[0-9a-f]{6}$/i.test(hex) ? parseInt(hex.slice(1), 16) : 0;
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${a})`;
}

/** CSS variables and classes for the look settings shared by every form. */
export function formLook(f: FormCfg): { className: string; style: React.CSSProperties } {
  return {
    className: (f.shadow === false ? " no-shadow" : "") + (f.showIcons === false ? " no-icons" : " is-" + (f.iconStyle ?? "tile")) + (f.showDelta === false ? " no-delta" : "") + (f.clickOpens === false ? " no-click" : ""),
    style: {
      ["--s" as string]: formScale(f),
      ["--bg" as string]: rgba(f.bg ?? "#000000", f.bgAlpha ?? 1),
      ["--r" as string]: f.roundness ?? 1,
      ["--sp" as string]: f.spacing ?? 1,
      ["--mid" as string]: f.gap ?? 26,
      opacity: f.opacity ?? 1,
    },
  };
}
