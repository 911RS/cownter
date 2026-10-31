import { useEffect, useRef, useState } from "react";
import { onCoreEvent } from "../core/api";
import { formatDelta, formatValue } from "../core/format";
import type { EntryView, FormCfg } from "../core/types";

interface Item {
  key: number;
  text: string;
  up: boolean;
  fx: string;
}

const LIFE_MS = 1600;

/** Plays the effect of a fired alert (`+1`, `-3`) over the widget showing that number. */
export function ChangeFx({ views }: { f?: FormCfg; views: EntryView[] }) {
  const [items, setItems] = useState<Item[]>([]);
  const seq = useRef(0);
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

  if (!items.length) return null;
  return (
    <div className="fx-layer" aria-hidden>
      {items.map((x) => (
        <span key={x.key} className={`fx fx-${x.fx} ${x.up ? "up" : "down"}`}>
          {x.fx === "burst" && (
            <span className="fx-sparks">
              {Array.from({ length: 8 }, (_, i) => (
                <i key={i} style={{ "--a": `${i * 45}deg` } as React.CSSProperties} />
              ))}
            </span>
          )}
          <b>{x.text}</b>
        </span>
      ))}
    </div>
  );
}
