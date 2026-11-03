import { useContext, useEffect, useState } from "react";
import { formatDelta, formatValue, goalLeft } from "../core/format";
import { DeltaHold, noteDelta } from "./deltaHold";
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
  const hold = useContext(DeltaHold);
  const at = noteDelta(v.entry.id, v.deltaToday);
  const left = hold > 0 ? at + hold * 1000 - Date.now() : Infinity;
  const [, tick] = useState(0);
  useEffect(() => {
    if (left <= 0 || left === Infinity) return;
    const t = setTimeout(() => tick((n) => n + 1), left + 50);
    return () => clearTimeout(t);
  }, [at, hold]);
  const t = formatDelta(v.deltaToday, v.entry.format);
  if (!t || left <= 0) return null;
  return <span className={"delta " + ((v.deltaToday ?? 0) > 0 ? "up" : "down")}>{t}</span>;
}

export function Streak({ v }: { v: EntryView }) {
  const n = v.streak ?? 0;
  if (n < 2) return null;
  return <span className="streak" title={`Up ${n} days in a row`}>🔥 {n}</span>;
}

export function GoalLeft({ v }: { v: EntryView }) {
  const t = goalLeft(v);
  if (!t) return null;
  return <span className="goal-left" title={`Goal ${formatValue(v.entry.goalValue, v.entry.format)}`}>{t}</span>;
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

/** Icon styles Shuffle picks from. */
export const SHUFFLE_STYLES = ["tile", "big", "fade", "glow", "glyph", "bar", "ghost", "tilt", "cover", "slant", "wash", "split", "sticker", "kinetic"] as const;

/** Icon style in effect; Shuffle moves to the next style each time `turn` changes. */
export function iconStyleOf(f: FormCfg, turn: number): string {
  const style = f.iconStyle ?? "tile";
  if (style !== "shuffle") return style;
  return SHUFFLE_STYLES[((turn % SHUFFLE_STYLES.length) + SHUFFLE_STYLES.length) % SHUFFLE_STYLES.length];
}

/** Counter that ticks every few seconds while the icon style is Shuffle. */
export function useShuffleTurn(f: FormCfg | null | undefined): number {
  const [turn, setTurn] = useState(() => Math.floor(Math.random() * SHUFFLE_STYLES.length));
  const on = f?.iconStyle === "shuffle";
  const every = Math.max(3, f?.cycleS ?? 4) * 1000;
  useEffect(() => {
    if (!on) return;
    const t = setInterval(() => setTurn((n) => n + 1), every);
    return () => clearInterval(t);
  }, [on, every]);
  return turn;
}

/** CSS variables and classes for the look settings shared by every form. */
export function formLook(f: FormCfg, turn = 0): { className: string; style: React.CSSProperties } {
  return {
    className: (f.shadow === false ? " no-shadow" : "") + (f.showIcons === false ? " no-icons" : " is-" + iconStyleOf(f, turn)) + (f.showDelta === false ? " no-delta" : "") + (f.clickOpens === false ? " no-click" : ""),
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
