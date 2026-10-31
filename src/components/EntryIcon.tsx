import type { Icon } from "../core/types";
import { BUILTIN_ICONS } from "./icons";

export function EntryIcon({ icon, size = 18 }: { icon: Icon; size?: number }) {
  const style = { width: size, height: size, background: icon.bg, borderRadius: size * 0.3 };
  let inner: React.ReactNode;
  if (icon.kind === "builtin") {
    const d = BUILTIN_ICONS[icon.value] ?? BUILTIN_ICONS.chart;
    inner = (
      <svg viewBox="0 0 24 24" width={size * 0.66} height={size * 0.66} fill="none" stroke="#fff" strokeWidth={2.2} strokeLinecap="round" strokeLinejoin="round">
        <path d={d} />
      </svg>
    );
  } else if (icon.kind === "image" || icon.kind === "favicon") {
    inner = <img src={icon.value} alt="" width={size * 0.72} height={size * 0.72} style={{ objectFit: "contain", borderRadius: size * 0.12 }} />;
  } else {
    inner = <span style={{ fontSize: size * 0.62, lineHeight: 1 }}>{icon.value || "📈"}</span>;
  }
  return (
    <span className="entry-icon" style={style}>
      {inner}
    </span>
  );
}
