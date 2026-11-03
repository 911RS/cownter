import { useEffect, useRef, useState } from "react";
import { api } from "../core/api";
import { formatDelta } from "../core/format";
import { useDisplay, useEntries, useSeason, useWidgetSize } from "../core/hooks";
import { SEASONS, seasonClass, seasonTint } from "../core/season";
import { selectEntries } from "../core/select";
import type { EntryView } from "../core/types";
import { EntryIcon } from "../components/EntryIcon";
import { Skin } from "../skins/Skin";
import { formLook, formScale, useShuffleTurn, valueText } from "./common";

const SHOW_MS = 4000;

interface Item {
  key: number;
  view: EntryView;
  diff: number;
  was?: string;
}

export function Pop() {
  const [cfg] = useDisplay();
  const season = useSeason(cfg?.seasonal);
  const cfgRef = useRef(cfg);
  cfgRef.current = cfg;
  const [queue, setQueue] = useState<Item[]>([]);
  const seq = useRef(0);
  useEntries((e, views) => {
    const c = cfgRef.current;
    if ((e.type !== "valueChanged" && e.type !== "textChanged") || !c?.pop.enabled) return;
    const view = views.get(e.entryId);
    if (!view || !selectEntries([view], c.pop).length) return;
    const item =
      e.type === "textChanged"
        ? { key: ++seq.current, view: { ...view, text: e.text }, diff: 0, was: e.prev }
        : { key: ++seq.current, view: { ...view, value: e.value }, diff: e.value - e.prev };
    setQueue((q) => [...q.filter((x) => x.view.entry.id !== e.entryId), item].slice(-5));
  });
  const [root, setRoot] = useState<HTMLDivElement | null>(null);
  const cur = queue[0] ?? null;

  useEffect(() => {
    if (!cur) {
      api.widgetDone().catch(() => {});
      return;
    }
    const t = setTimeout(() => setQueue((q) => q.slice(1)), SHOW_MS);
    return () => clearTimeout(t);
  }, [cur?.key]);

  useWidgetSize(cur ? root : null, true);
  const turn = useShuffleTurn(cfg?.pop);
  if (!cfg || !cur) return null;
  const f = cfg.pop;
  const s = formScale(f);
  const look = formLook(f, turn);
  const mark = season && SEASONS[season].mark;
  const d = cur.was !== undefined ? `was ${cur.was.length > 24 ? cur.was.slice(0, 23) + "…" : cur.was}` : formatDelta(cur.diff, cur.view.entry.format);
  return (
    <div className={"widget-root pop-root" + look.className + seasonClass(season)} ref={setRoot} key={cur.key} style={look.style}>
      <div className={"pop" + (cur.was !== undefined ? "" : cur.diff > 0 ? " up" : cur.diff < 0 ? " down" : "")} onClick={() => setQueue((q) => q.slice(1))}>
        <EntryIcon icon={cur.view.entry.icon} size={34 * s} />
        <div className="pop-body">
          <span className="pop-title">
            <b className={cur.was !== undefined ? "muted" : cur.diff > 0 ? "up" : "down"}>{d}</b> {cur.view.entry.name}
            {mark && <span className="season-mark">{mark}</span>}
          </span>
          <Skin skin={f.skin} tint={seasonTint(season, f.tint)} opts={f.skinOpts} text={valueText(cur.view)} height={26 * s} />
        </div>
      </div>
    </div>
  );
}
