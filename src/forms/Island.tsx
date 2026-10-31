import { useEffect, useRef, useState } from "react";
import { api } from "../core/api";
import { useDisplay, useEntries, useSeason, useWidgetSize } from "../core/hooks";
import { SEASONS, seasonClass, seasonTint } from "../core/season";
import { selectEntries } from "../core/select";
import { EntryIcon } from "../components/EntryIcon";
import { Morph } from "../components/Morph";
import { Skin } from "../skins/Skin";
import { ChangeFx } from "./ChangeFx";
import { formOptClasses } from "./options";
import { Delta, formLook, formScale, GoalLeft, StatusMark, Streak, valueText } from "./common";

export function Island() {
  const [cfg] = useDisplay();
  const season = useSeason(cfg?.seasonal);
  const [pulse, setPulse] = useState<number | null>(null);
  const listRef = useRef<number[]>([]);
  const views = useEntries((e) => {
    if (e.type === "valueChanged") {
      const i = listRef.current.indexOf(e.entryId);
      if (i >= 0) {
        setIdx(i);
        setPulse(e.entryId);
      }
    }
  });
  const [hover, setHover] = useState(false);
  const [idx, setIdx] = useState(0);
  const [root, setRoot] = useState<HTMLDivElement | null>(null);
  const leave = useRef<ReturnType<typeof setTimeout>>(undefined);

  const f = cfg?.island;
  const list = f ? selectEntries([...views.values()], f) : [];
  const n = list.length;
  listRef.current = list.map((v) => v.entry.id);

  useEffect(() => {
    if (pulse === null) return;
    const t = setTimeout(() => setPulse(null), 1400);
    return () => clearTimeout(t);
  }, [pulse]);

  useEffect(() => {
    if (hover || n < 2 || !f) return;
    const t = setInterval(() => setIdx((i) => (i + 1) % n), f.cycleS * 1000);
    return () => clearInterval(t);
  }, [hover, n, f]);

  useWidgetSize(root, !!f?.enabled);
  if (!cfg || !f) return null;
  const s = formScale(f);
  const look = formLook(f);
  const tint = seasonTint(season, f.tint);
  const mark = season && SEASONS[season].mark;
  const cur = n ? list[idx % n] : null;

  const onEnter = () => {
    if (f.hoverExpands === false) return;
    clearTimeout(leave.current);
    setHover(true);
  };
  const onLeave = () => {
    leave.current = setTimeout(() => setHover(false), 350);
  };

  return (
    <div className={"widget-root island-root" + look.className + seasonClass(season) + formOptClasses("island", f)} ref={setRoot} style={look.style}>
      <ChangeFx f={f} views={list} />
      <Morph
        live
        open={hover && n > 0}
        position={f.position}
        className={"island" + (pulse !== null ? " pulse" : "")}
        onEnter={onEnter}
        onLeave={onLeave}
        onClick={() => f.clickOpens !== false && api.openSettings()}
        title={f.clickOpens !== false ? "Open Cownter" : undefined}
        closed={
          cur ? (
            <div className="island-row" key={cur.entry.id}>
              <EntryIcon icon={cur.entry.icon} size={18 * s} />
              <Skin skin={f.skin} tint={tint} opts={f.skinOpts} text={valueText(cur)} height={19 * s} status={cur.status} />
              <Delta v={cur} />
              <StatusMark status={cur.status} error={cur.error} />
              {mark && <span className="season-mark">{mark}</span>}
              {n > 1 && f.showDots !== false && (
                <span className="dots">
                  {list.slice(0, 6).map((v, i) => (
                    <i key={v.entry.id} className={i === idx % n ? "on" : ""} />
                  ))}
                </span>
              )}
            </div>
          ) : (
            <span className="island-empty">Cownter · add your first number</span>
          )
        }
        opened={
          n > 0 ? (
            <div className="island-grid" style={{ gridTemplateColumns: `repeat(${Math.min(3, n)}, auto)` }}>
              {list.map((v) => (
                <div className={"chip" + (pulse === v.entry.id ? " pulse" : "")} key={v.entry.id} title={v.error ?? v.entry.name}>
                  <EntryIcon icon={v.entry.icon} size={20 * s} />
                  <div className="chip-body">
                    <Skin skin={f.skin} tint={tint} opts={f.skinOpts} text={valueText(v)} height={17 * s} status={v.status} />
                    <span className="chip-label">
                      {v.entry.name} <Delta v={v} />
                      <Streak v={v} />
                      <GoalLeft v={v} />
                      <StatusMark status={v.status} error={v.error} />
                    </span>
                  </div>
                </div>
              ))}
            </div>
          ) : null
        }
      />
    </div>
  );
}
