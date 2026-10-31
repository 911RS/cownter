import type { Icon } from "../core/types";
import { BUILTIN_ICONS } from "./icons";

function light(hex: string): boolean {
  const m = /^#([0-9a-f]{6})$/i.exec(hex);
  if (!m) return false;
  const n = parseInt(m[1], 16);
  return ((n >> 16) & 255) * 0.299 + ((n >> 8) & 255) * 0.587 + (n & 255) * 0.114 > 170;
}

function dark(hex: string): boolean {
  const m = /^#([0-9a-f]{6})$/i.exec(hex);
  if (!m) return true;
  const n = parseInt(m[1], 16);
  return ((n >> 16) & 255) * 0.299 + ((n >> 8) & 255) * 0.587 + (n & 255) * 0.114 < 70;
}

export function EntryIcon({ icon, size = 18 }: { icon: Icon; size?: number }) {
  const style = {
    width: size,
    height: size,
    background: icon.bg,
    borderRadius: size * 0.3,
    ["--ink" as string]: light(icon.bg) ? "#14161b" : "#fff",
    ["--accent" as string]: dark(icon.bg) ? "#fff" : icon.bg,
    ["--hue" as string]: dark(icon.bg) ? "#8b93a7" : icon.bg,
  };
  let inner: React.ReactNode;
  if (icon.kind === "builtin") {
    const d = BUILTIN_ICONS[icon.value] ?? BUILTIN_ICONS.chart;
    inner = (
      <svg viewBox="0 0 24 24" width={size * 0.66} height={size * 0.66} fill="none" stroke="var(--ink)" strokeWidth={2.2} strokeLinecap="round" strokeLinejoin="round">
        <path d={d} />
      </svg>
    );
  } else if (icon.kind === "image" || icon.kind === "favicon") {
    inner = <img src={icon.value} alt="" width={size * 0.72} height={size * 0.72} style={{ objectFit: "contain", borderRadius: size * 0.12 }} />;
  } else {
    inner = <span style={{ fontSize: size * 0.62, lineHeight: 1 }}>{icon.value || "📈"}</span>;
  }
  return (
    <span className={"entry-icon ei-" + icon.kind} style={style}>
      {inner}
    </span>
  );
}
