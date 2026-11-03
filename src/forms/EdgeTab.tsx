import { useRef, useState } from "react";
import { api } from "../core/api";
import { useDisplay, useEntries, useSeason, useWidgetSize } from "../core/hooks";
import { seasonClass, seasonTint } from "../core/season";
import { selectEntries } from "../core/select";
import { EntryIcon } from "../components/EntryIcon";
import { Skin } from "../skins/Skin";
import { DeltaHold, noteDeltas } from "./deltaHold";
import { ChangeFx } from "./ChangeFx";
import { formOptClasses } from "./options";
import { Delta, formLook, formScale, useShuffleTurn, StatusMark, valueText } from "./common";

export function EdgeTab() {
  const [cfg] = useDisplay();
  const season = useSeason(cfg?.seasonal);
  const [pulse, setPulse] = useState<number | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const views = useEntries((e) => {
    if (e.type === "valueChanged") {
      setPulse(e.entryId);
      clearTimeout(timer.current);
      timer.current = setTimeout(() => setPulse(null), 1400);
    }
  });
  const [open, setOpen] = useState(false);
  const leave = useRef<ReturnType<typeof setTimeout>>(undefined);
  const [root, setRoot] = useState<HTMLDivElement | null>(null);
  const f = cfg?.edge;
  useWidgetSize(root, !!f?.enabled);
  const turn = useShuffleTurn(f);
  if (!cfg || !f) return null;
  const s = formScale(f);
  const look = formLook(f, turn);
  const tint = seasonTint(season, f.tint);
  const list = selectEntries([...views.values()], f);

  noteDeltas(list);
  return (
    <DeltaHold.Provider value={f.deltaHoldS ?? 0}>
    <div className={"widget-root edge-root side-" + f.side + look.className + seasonClass(season) + formOptClasses("edge", f)} ref={setRoot} style={look.style}>
      <ChangeFx f={f} views={list} />
      <div
        className={"edge" + (open ? " open" : "")}
        onMouseEnter={() => {
          if (f.hoverExpands === false) return;
          clearTimeout(leave.current);
          setOpen(true);
        }}
        onMouseLeave={() => {
          leave.current = setTimeout(() => setOpen(false), 350);
        }}
      >
        {list.length === 0 && (
          <button className="edge-empty" onClick={() => api.openSettings()} title="Add a number">
            +
          </button>
        )}
        {list.map((v) => (
          <div
            className={"edge-row" + (pulse === v.entry.id ? " pulse" : "")}
            key={v.entry.id}
            title={v.error ?? v.entry.name}
            onClick={() => f.clickOpens !== false && api.openSettings()}
          >
            <EntryIcon icon={v.entry.icon} size={18 * s} />
            {open ? (
              <>
                <span className="edge-name">{v.entry.name}</span>
                <Skin skin={f.skin} tint={tint} opts={f.skinOpts} text={valueText(v)} height={16 * s} status={v.status} />
                <Delta v={v} />
                <StatusMark status={v.status} error={v.error} />
              </>
            ) : (
              <Skin skin={f.skin} tint={tint} opts={f.skinOpts} text={valueText(v, true)} height={12 * s} status={v.status} />
            )}
          </div>
        ))}
      </div>
    </div>
    </DeltaHold.Provider>
  );
}
