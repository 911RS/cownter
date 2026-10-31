import { useEffect, useId, useRef, useState } from "react";
import { glyphFor } from "./glyphs";
import { skinOpt } from "./options";

const PITCH = 10;
const R = 4.6;

interface Props {
  text: string;
  /** Height of the digits in px. */
  height: number;
  opts?: Record<string, boolean>;
}

export function DotMatrix({ text, height, opts }: Props) {
  const square = skinOpt("dotMatrix", opts, "square");
  const panel = skinOpt("dotMatrix", opts, "panel");
  const [flash, setFlash] = useState(false);
  const fid = "dmg" + useId().replace(/[^a-zA-Z0-9]/g, "");
  const prev = useRef(text);
  useEffect(() => {
    if (prev.current !== text) {
      prev.current = text;
      setFlash(true);
      const t = setTimeout(() => setFlash(false), 420);
      return () => clearTimeout(t);
    }
  }, [text]);

  const chars = [...text];
  const cols: { x: number; rows: string[] }[] = [];
  let x = 0;
  for (const ch of chars) {
    const rows = glyphFor(ch);
    cols.push({ x, rows });
    x += rows[0].length + 1;
  }
  const w = Math.max(1, x - 1) * PITCH;
  const h = 7 * PITCH;
  const lit: string[] = [];
  const off: string[] = [];
  const hot: string[] = [];
  const dot = (cx: number, cy: number, r: number) =>
    square ? `M${cx - r} ${cy - r}h${r * 2}v${r * 2}h${-r * 2}z` : `M${cx - r} ${cy}a${r} ${r} 0 1 0 ${r * 2} 0a${r} ${r} 0 1 0 ${-r * 2} 0`;
  cols.forEach(({ x: gx, rows }) =>
    rows.forEach((row, ry) =>
      [...row].forEach((c, rx) => {
        const cx = (gx + rx) * PITCH + PITCH / 2;
        const cy = ry * PITCH + PITCH / 2;
        if (c === "#") {
          lit.push(dot(cx, cy, R));
          hot.push(dot(cx - R * 0.3, cy - R * 0.3, R * 0.38));
        } else off.push(dot(cx, cy, R));
      }),
    ),
  );
  const pad = panel ? Math.max(2, height * 0.16) : 0;
  return (
    <span className={"dm-panel" + (flash ? " dm-flash" : "")} style={{ padding: `${pad}px ${pad * 1.2}px`, borderRadius: pad }}>
      <svg className="dm" viewBox={`0 0 ${w} ${h}`} height={height} width={(height * w) / h} role="img" aria-label={text}>
        <defs>
          <filter id={fid} x="-20%" y="-20%" width="140%" height="140%">
            <feGaussianBlur stdDeviation="3" />
          </filter>
        </defs>
        <path className="dm-off" d={off.join("")} />
        <path className="dm-glow" d={lit.join("")} filter={`url(#${fid})`} />
        <path className="dm-on" d={lit.join("")} />
        <path className="dm-hot" d={hot.join("")} />
      </svg>
    </span>
  );
}
