import { useEffect, useRef, useState } from "react";
import { onCoreEvent } from "../core/api";
import { formatDelta, formatValue } from "../core/format";
import type { AlertEffect, EntryView, FormCfg } from "../core/types";

interface Item {
  key: number;
  text: string;
  up: boolean;
  fx: AlertEffect;
}

const LIFE_MS = 1800;
/** Effects that move or light up the whole widget instead of drawing text over it. */
export const WIDGET_FX: AlertEffect[] = ["shake", "glow", "flash"];
const CONFETTI = ["#ff5a5a", "#ffd24d", "#5dff8a", "#5ad1ff", "#b277ff", "#ff7ad9"];

/** One effect, drawn over a widget or in the alert preview. */
export function FxItem({ fx, text, up }: { fx: AlertEffect; text: string; up: boolean }) {
  if (!fx || WIDGET_FX.includes(fx)) return null;
  const dir = up ? "up" : "down";
  if (fx === "rain") {
    return (
      <span className={`fx fx-rain ${dir}`}>
        {Array.from({ length: 6 }, (_, i) => (
          <b key={i} style={{ "--x": `${(i - 2.5) * 22}px`, "--d": `${i * 0.12}s` } as React.CSSProperties}>
            {text}
          </b>
        ))}
      </span>
    );
  }
  return (
    <span className={`fx fx-${fx} ${dir}`}>
      {fx === "burst" && (
        <span className="fx-sparks">
          {Array.from({ length: 8 }, (_, i) => (
            <i key={i} style={{ "--a": `${i * 45}deg` } as React.CSSProperties} />
          ))}
        </span>
      )}
      {fx === "confetti" && (
        <span className="fx-bits">
          {Array.from({ length: 16 }, (_, i) => (
            <i
              key={i}
              style={
                {
                  "--x": `${Math.round(Math.cos(i * 2.4) * 46)}px`,
                  "--y": `${Math.round(18 + ((i * 37) % 30))}px`,
                  "--r": `${(i * 67) % 360}deg`,
                  background: CONFETTI[i % CONFETTI.length],
                } as React.CSSProperties
              }
            />
          ))}
        </span>
      )}
      <b>{text}</b>
    </span>
  );
}

/** Plays the effect of a fired alert (`+1`, `-3`) over the widget showing that number. */
export function ChangeFx({ views }: { f?: FormCfg; views: EntryView[] }) {
  const [items, setItems] = useState<Item[]>([]);
  const seq = useRef(0);
  const layer = useRef<HTMLDivElement>(null);
  const ref = useRef({ views });
  ref.current = { views };

  useEffect(() => {
    const timers = new Set<ReturnType<typeof setTimeout>>();
    const un = onCoreEvent((e) => {
      if (e.type !== "alertFired" || !e.effect) return;
      const v = ref.current.views.find((x) => x.entry.id === e.entryId);
      if (!v) return;
      const diff = e.diff ?? 0;
      const text = e.diff !== null && e.diff !== 0 ? formatDelta(e.diff, v.entry.format) : formatValue(e.value, v.entry.format);
      const item = { key: ++seq.current, text, up: diff >= 0, fx: e.effect };
      setItems((l) => [...l.slice(-3), item]);
      const root = layer.current?.parentElement;
      if (root && WIDGET_FX.includes(e.effect)) {
        const cls = [`fx-${e.effect}-on`, diff >= 0 ? "fx-up" : "fx-down"];
        root.classList.remove(...cls);
        void root.offsetWidth;
        root.classList.add(...cls);
        const t = setTimeout(() => {
          timers.delete(t);
          root.classList.remove(...cls);
        }, 1300);
        timers.add(t);
      }
      const t = setTimeout(() => {
        timers.delete(t);
        setItems((l) => l.filter((x) => x.key !== item.key));
      }, LIFE_MS);
      timers.add(t);
    });
    return () => {
      un.then((u) => u());
      timers.forEach(clearTimeout);
    };
  }, []);

  return (
    <div className="fx-layer" aria-hidden ref={layer}>
      {items.map((x) => (
        <FxItem key={x.key} fx={x.fx} text={x.text} up={x.up} />
      ))}
    </div>
  );
}
