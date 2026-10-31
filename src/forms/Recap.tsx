import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";
import { api } from "../core/api";
import { dateRange, formatDelta, formatValue, weekday } from "../core/format";
import { useDisplay, useWidgetSize } from "../core/hooks";
import type { Recap as RecapData } from "../core/types";
import { EntryIcon } from "../components/EntryIcon";
import { formLook, formScale } from "./common";

const SHOW_MS = 15000;

export function Recap() {
  const [cfg] = useDisplay();
  const [recap, setRecap] = useState<{ key: number; data: RecapData } | null>(null);
  const seq = useRef(0);
  const [root, setRoot] = useState<HTMLDivElement | null>(null);

  useEffect(() => {
    const load = () =>
      api
        .weeklyRecap()
        .then((data) => setRecap({ key: ++seq.current, data }))
        .catch(() => setRecap(null));
    load();
    const un = listen("recap", load);
    return () => {
      un.then((u) => u());
    };
  }, []);

  useEffect(() => {
    if (!recap) {
      api.widgetDone().catch(() => {});
      return;
    }
    const t = setTimeout(() => setRecap(null), SHOW_MS);
    return () => clearTimeout(t);
  }, [recap?.key]);

  useWidgetSize(recap && cfg ? root : null, true);
  if (!cfg || !recap) return null;
  const f = cfg.pop;
  const s = formScale(f);
  const look = formLook(f);
  const { since, until, items } = recap.data;
  return (
    <div className={"widget-root recap-root" + look.className} ref={setRoot} key={recap.key} style={look.style}>
      <div className="recap" onClick={() => setRecap(null)}>
        <div className="recap-head">
          <b>Your week</b>
          <span>{dateRange(since, until)}</span>
        </div>
        {items.length === 0 && <p className="recap-empty">No numbers recorded this week yet.</p>}
        {items.map((it) => (
          <div className="recap-row" key={it.entry.id}>
            <EntryIcon icon={it.entry.icon} size={26 * s} />
            <span className="recap-name">
              <b>{it.entry.name}</b>
              <small>{formatValue(it.value, it.entry.format)}</small>
            </span>
            <span className={"recap-change " + (it.change > 0 ? "up" : it.change < 0 ? "down" : "muted")}>{formatDelta(it.change, it.entry.format) || "±0"}</span>
            <span className="recap-best">{it.bestDay ? `Best ${weekday(it.bestDay.day)} ${formatDelta(it.bestDay.gain, it.entry.format)}` : ""}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
