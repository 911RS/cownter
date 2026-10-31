import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";
import { api } from "../core/api";
import { formatDelta, timeAgo } from "../core/format";
import { useDisplay, useEntries, useNow, useWidgetSize } from "../core/hooks";
import { selectEntries } from "../core/select";
import type { EntryView, FormCfg } from "../core/types";
import { EntryIcon } from "../components/EntryIcon";
import { Morph } from "../components/Morph";
import { Spark } from "../components/Spark";
import { Skin } from "../skins/Skin";
import { ChangeFx, FxItem } from "./ChangeFx";
import { PET_STAGES, petStage, petTitle } from "./pet";
import { formOpt, formOptClasses } from "./options";
import { Delta, formLook, formScale, GoalLeft, StatusMark, Streak, valueText } from "./common";

const PEEK_MS = 4000;
const FLUSH = ["notch", "wings", "toptabs", "folders", "tag", "clothesline", "deck"];

/** Screen edge a flush form is attached to, from its anchor. */
export function edgeOf(id: string, f: FormCfg): string {
  if (!FLUSH.includes(id)) return "";
  const p = f.position;
  if (p.startsWith("top")) return " edge-top";
  if (p.startsWith("bottom")) return " edge-bottom";
  if (p === "middleLeft") return " edge-left";
  if (p === "middleRight") return " edge-right";
  return "";
}

interface Change {
  key: number;
  view: EntryView;
  diff: number;
  at: number;
}

interface PetUp {
  key: number;
  entryId: number;
  stage: number;
}

interface Ctx {
  f: FormCfg;
  s: number;
  list: EntryView[];
  cur: EntryView | null;
  idx: number;
  open: boolean;
  /** True in a real widget window, false in settings previews. */
  live: boolean;
  leave?: () => void;
  pulse: number | null;
  feed: Change[];
  now: number;
  trip: number;
  petUp?: PetUp | null;
  num: (v: EntryView, h: number, compact?: boolean) => React.ReactElement;
  icon: (v: EntryView, size: number) => React.ReactElement | null;
}

const FOLDER_COLORS = ["#f2b84b", "#5aa9ff", "#6fd08c", "#ff8a7a", "#b48cff", "#4fd1c5"];

function folderColor(v: EntryView, i: number): string {
  const bg = v.entry.icon.bg;
  if (!/^#[0-9a-f]{6}$/i.test(bg)) return FOLDER_COLORS[i % FOLDER_COLORS.length];
  const n = parseInt(bg.slice(1), 16);
  const lum = (0.299 * ((n >> 16) & 255) + 0.587 * ((n >> 8) & 255) + 0.114 * (n & 255)) / 255;
  return lum < 0.35 ? FOLDER_COLORS[i % FOLDER_COLORS.length] : bg;
}

function Dots({ c }: { c: Ctx }) {
  if (!c.f.showDots || c.list.length < 2) return null;
  return (
    <span className="dots">
      {c.list.slice(0, 8).map((v, i) => (
        <i key={v.entry.id} className={i === c.idx ? "on" : ""} />
      ))}
    </span>
  );
}

function Name({ c, v }: { c: Ctx; v: EntryView }) {
  return c.f.showNames ? <span className="b-name">{v.entry.name}</span> : null;
}

function Cell({ c, v, h = 15 }: { c: Ctx; v: EntryView; h?: number }) {
  return (
    <div className={"b-cell" + (c.pulse === v.entry.id ? " pulse" : "")} title={v.error ?? v.entry.name}>
      {c.icon(v, 18 * c.s)}
      <div className="b-stack">
        {c.num(v, h * c.s)}
        <span className="b-sub">
          <Name c={c} v={v} />
          <Delta v={v} />
          <StatusMark status={v.status} error={v.error} />
        </span>
      </div>
    </div>
  );
}

function List({ c, h = 14 }: { c: Ctx; h?: number }) {
  return (
    <div className="b-list">
      {c.list.map((v) => (
        <div className={"b-row" + (c.pulse === v.entry.id ? " pulse" : "")} key={v.entry.id} title={v.error ?? v.entry.name}>
          {c.icon(v, 16 * c.s)}
          <Name c={c} v={v} />
          <GoalLeft v={v} />
          <span className="b-grow" />
          {c.num(v, h * c.s)}
          <Delta v={v} />
          <Streak v={v} />
        </div>
      ))}
    </div>
  );
}

const PAPER = ["#ffe58a", "#ffb3c7", "#a7e3ff", "#b8f5b0", "#e0c3ff", "#ffd2a1"];

const LAYOUTS: Record<string, (c: Ctx) => React.ReactNode> = {
  bar: (c) => (
    <div className="b-panel b-bar">
      {c.list.map((v, i) => (
        <div className="b-seg" key={v.entry.id}>
          {i > 0 && <span className="b-div" />}
          <Cell c={c} v={v} />
        </div>
      ))}
    </div>
  ),
  pills: (c) => (
    <div className="b-pills">
      {c.list.map((v) => (
        <div className="b-panel b-pill" key={v.entry.id}>
          <Cell c={c} v={v} />
        </div>
      ))}
    </div>
  ),
  deck: (c) => (
    <div className="b-panel b-deck">
      {c.list.map((v, i) => (
        <div className="b-seg" key={v.entry.id}>
          {i > 0 && <span className="b-div" />}
          <Cell c={c} v={v} />
        </div>
      ))}
    </div>
  ),
  orb: (c) =>
    c.open ? (
      <div className="b-panel b-orb-open">
        <List c={c} />
      </div>
    ) : (
      <div className="b-panel b-orb">
        {c.cur && c.icon(c.cur, 16 * c.s)}
        {c.cur && c.num(c.cur, 12 * c.s, true)}
      </div>
    ),
  ticker: (c) => (
    <div className="b-panel b-ticker">
      <div className="b-track" style={{ animationDuration: `${Math.max(8, c.list.length * 5)}s` }}>
        {[0, 1].map((k) => (
          <div className="b-run" key={k} aria-hidden={k === 1}>
            {c.list.map((v) => (
              <span className="b-tick" key={v.entry.id}>
                {c.icon(v, 14 * c.s)}
                <Name c={c} v={v} />
                {c.num(v, 12 * c.s)}
                <Delta v={v} />
                <span className="b-sep">•</span>
              </span>
            ))}
          </div>
        ))}
      </div>
    </div>
  ),
  trend: (c) => (
    <div className="b-panel b-trend">
      {c.list.map((v) => (
        <div className={"b-row" + (c.pulse === v.entry.id ? " pulse" : "")} key={v.entry.id}>
          {c.icon(v, 16 * c.s)}
          <Name c={c} v={v} />
          <span className="b-grow" />
          <Spark points={v.spark} width={64 * c.s} height={18 * c.s} color={(v.deltaToday ?? 0) < 0 ? "#ff7a6b" : "#7dff9b"} />
          {c.num(v, 13 * c.s)}
        </div>
      ))}
    </div>
  ),
  chips: (c) => (
    <div className="b-stickies">
      {c.list.map((v, i) => (
        <div
          className={"b-sticky" + (c.pulse === v.entry.id ? " pulse" : "")}
          key={v.entry.id}
          style={{ background: PAPER[i % PAPER.length], transform: `rotate(${i % 2 ? 2 : -2}deg)` }}
        >
          <span className="b-sticky-head">
            {c.icon(v, 14 * c.s)}
            <Name c={c} v={v} />
          </span>
          <span className="b-screen">{c.num(v, 14 * c.s)}</span>
          <Delta v={v} />
        </div>
      ))}
    </div>
  ),
  versus: (c) => {
    const [a, b] = c.list;
    if (!a || !b) return <div className="b-panel b-empty">Pick two numbers for Versus</div>;
    const total = Math.abs(a.value ?? 0) + Math.abs(b.value ?? 0);
    const share = total ? Math.abs(a.value ?? 0) / total : 0.5;
    return (
      <div className="b-panel b-versus">
        <div className="b-vs-row">
          <Cell c={c} v={a} h={18} />
          <span className="b-vs">VS</span>
          <Cell c={c} v={b} h={18} />
        </div>
        <div className="b-vs-bar">
          <i style={{ width: `${share * 100}%` }} />
        </div>
      </div>
    );
  },
  stream: (c) => (
    <div className="b-panel b-stream">
      {c.list.map((v) => (
        <div className={"b-stream-cell" + (c.pulse === v.entry.id ? " pulse" : "")} key={v.entry.id}>
          <span className="b-stream-name">
            {c.icon(v, 18 * c.s)}
            <Name c={c} v={v} />
          </span>
          {c.num(v, 30 * c.s)}
          <Delta v={v} />
        </div>
      ))}
    </div>
  ),
  peek: (c) => (
    <div className="b-panel b-peek">
      <div className="b-peek-grid" style={{ gridTemplateColumns: `repeat(${Math.min(3, Math.max(1, c.list.length))}, auto)` }}>
        {c.list.map((v) => (
          <Cell c={c} v={v} h={22} key={v.entry.id} />
        ))}
      </div>
    </div>
  ),
  drawer: (c) => (
    <div className={"b-drawer side-" + c.f.side}>
      {c.open && (
        <div className="b-panel b-drawer-body">
          <List c={c} />
        </div>
      )}
      <div className="b-panel b-handle">
        <i />
        <i />
        <i />
      </div>
    </div>
  ),
  notch: (c) => (
    <Morph
      live={c.live}
      open={c.open}
      position={c.f.position}
      className="b-panel b-notch"
      onLeave={c.leave}
      closed={
        c.cur ? (
          <div className="b-line">
            {c.icon(c.cur, 16 * c.s)}
            {c.num(c.cur, 14 * c.s)}
            <Delta v={c.cur} />
            <Dots c={c} />
          </div>
        ) : null
      }
      opened={c.list.length ? <List c={c} /> : null}
    />
  ),
  wings: (c) => {
    const half = Math.ceil(c.list.length / 2);
    return (
      <div className="b-wings">
        <div className="b-panel b-notch b-wing left">
          {c.list.slice(0, half).map((v) => (
            <Cell c={c} v={v} key={v.entry.id} />
          ))}
        </div>
        <div className="b-gap" />
        <div className="b-panel b-notch b-wing right">
          {c.list.slice(half).map((v) => (
            <Cell c={c} v={v} key={v.entry.id} />
          ))}
        </div>
      </div>
    );
  },
  toptabs: (c) => (
    <div className="b-tabs">
      <span className="b-tabs-strip" />
      <div className="b-tabs-row">
        {c.list.map((v, i) =>
          i === c.idx ? (
            <div className={"b-tab on" + (c.pulse === v.entry.id ? " pulse" : "")} key={v.entry.id} title={v.error ?? v.entry.name}>
              {c.icon(v, 18 * c.s)}
              <div className="b-stack">
                {c.num(v, 15 * c.s)}
                <span className="b-sub">
                  <Name c={c} v={v} />
                  <Delta v={v} />
                  <StatusMark status={v.status} error={v.error} />
                </span>
              </div>
            </div>
          ) : (
            <div className="b-tab" key={v.entry.id} title={v.entry.name}>
              {c.icon(v, 13 * c.s)}
              {c.num(v, 9 * c.s, true)}
            </div>
          ),
        )}
      </div>
    </div>
  ),
  folders: (c) => (
    <div className="b-folders">
      {c.list.map((v, i) => (
        <div
          className={"b-folder" + (c.pulse === v.entry.id ? " pulse" : "")}
          key={v.entry.id}
          style={{ zIndex: c.list.length - i, ["--fc" as string]: folderColor(v, i) }}
          title={v.error ?? v.entry.name}
        >
          <div className="b-folder-body">
            {c.icon(v, 16 * c.s)}
            {c.num(v, 14 * c.s)}
            <Delta v={v} />
            <StatusMark status={v.status} error={v.error} />
          </div>
          <span className="b-folder-tab" style={{ marginLeft: `calc(${8 + (i % 3) * 16}px * var(--s))` }}>
            {c.f.showNames ? v.entry.name : "\u00a0"}
          </span>
        </div>
      ))}
    </div>
  ),
  tag: (c) =>
    c.cur && (
      <div className={"b-tag-wrap" + (c.pulse !== null ? " swing" : "")}>
        <span className="b-string" />
        <div className="b-panel b-tag">
          <span className="b-hole" />
          {c.icon(c.cur, 18 * c.s)}
          {c.num(c.cur, 15 * c.s)}
          <span className="b-sub">
            <Name c={c} v={c.cur} />
            <Delta v={c.cur} />
          </span>
          <Dots c={c} />
        </div>
      </div>
    ),
  clothesline: (c) => (
    <div className="b-line-wrap">
      <svg className="b-rope" viewBox="0 0 100 10" preserveAspectRatio="none" aria-hidden>
        <path d="M0 1 Q50 9 100 1" />
      </svg>
      <div className="b-cards">
        {c.list.map((v, i) => (
          <div className={"b-hang" + (c.pulse === v.entry.id ? " swing" : "")} key={v.entry.id} style={{ transform: `rotate(${(i % 3) - 1}deg)` }}>
            <span className="b-peg" />
            <div className="b-panel b-card">
              {c.icon(v, 16 * c.s)}
              {c.num(v, 13 * c.s)}
              <Name c={c} v={v} />
              <Delta v={v} />
            </div>
          </div>
        ))}
      </div>
    </div>
  ),
  train: (c) => (
    <div className="b-rail moving" style={{ "--trip": `${c.trip}s` } as React.CSSProperties}>
      <div className={"b-train" + (c.pulse !== null ? " chug" : "")}>
        {c.list.map((v) => (
          <div className="b-wagon" key={v.entry.id}>
            <div className="b-panel b-wagon-body">
              {c.icon(v, 14 * c.s)}
              {c.num(v, 12 * c.s)}
            </div>
            <span className="b-wheels">
              <i />
              <i />
            </span>
          </div>
        ))}
        <div className="b-loco">
          <div className="b-panel b-loco-body">
            <span className="b-chimney">
              <i />
              <i />
              <i />
            </span>
            <span className="b-cab" />
          </div>
          <span className="b-wheels">
            <i />
            <i />
            <i />
          </span>
        </div>
      </div>
    </div>
  ),
  pet: (c) => {
    const last = c.feed[0];
    const v = last && c.now - last.at < 15000 ? last.view : c.cur;
    const live = v && c.list.find((x) => x.entry.id === v.entry.id);
    const stage = live ? petStage(live) : 3;
    const up = c.petUp && v && c.petUp.entryId === v.entry.id ? c.petUp : null;
    return (
      <div className={"b-pet" + (c.pulse !== null ? " hop" : "") + (up ? " grew" : "")}>
        {v && (
          <div className="b-panel b-bubble">
            {c.icon(v, 14 * c.s)}
            {c.num(v, 13 * c.s)}
            {last && v === last.view ? <span className={last.diff > 0 ? "up" : "down"}>{formatDelta(last.diff, v.entry.format)}</span> : <Delta v={v} />}
          </div>
        )}
        <span className="b-pet-body" title={live ? petTitle(live) : undefined}>
          <Cow stage={stage} />
          {up && (
            <span className="fx-layer b-pet-fx" key={up.key}>
              <FxItem fx="confetti" text={PET_STAGES[up.stage].label + "!"} up />
            </span>
          )}
        </span>
      </div>
    );
  },
  pager: (c) => (
    <div className={"b-pager" + (c.cur && c.pulse === c.cur.entry.id ? " ping" : "")}>
      <div className="b-pager-grille">
        <i />
        <i />
        <i />
        <i />
      </div>
      <div className="b-pager-screen">
        {c.cur && (
          <>
            <div className="b-pager-line">
              <span className="b-pager-name">{c.f.showNames ? c.cur.entry.name : ""}</span>
              <span className="b-pager-time">{timeAgo(c.cur.updatedAt, c.now)}</span>
            </div>
            <div className="b-pager-num">{c.num(c.cur, 18 * c.s)}</div>
            <div className="b-pager-line">
              <Delta v={c.cur} />
              <StatusMark status={c.cur.status} error={c.cur.error} />
              <span className="b-grow" />
              {c.f.showDots && c.list.length > 1 && (
                <span className="b-pager-page">
                  {c.idx + 1}/{c.list.length}
                </span>
              )}
            </div>
          </>
        )}
      </div>
      <div className="b-pager-side">
        <i className="b-pager-led" />
        <i className="b-pager-btn" />
      </div>
    </div>
  ),
  feed: (c) => (
    <div className="b-panel b-feed">
      {c.feed.length === 0 && <span className="b-muted">Waiting for changes…</span>}
      {c.feed.slice(0, 6).map((x) => (
        <div className="b-row" key={x.key}>
          {c.icon(x.view, 14 * c.s)}
          <b className={x.diff > 0 ? "up" : "down"}>{formatDelta(x.diff, x.view.entry.format)}</b>
          <Name c={c} v={x.view} />
          <span className="b-grow" />
          <span className="b-muted">{timeAgo(x.at, c.now)}</span>
        </div>
      ))}
    </div>
  ),
};

function Cow({ stage = 3 }: { stage?: number }) {
  const id = PET_STAGES[stage]?.id ?? "adult";
  if (id === "egg") {
    return (
      <svg className="b-cow st-egg" viewBox="0 0 64 56" aria-hidden>
        <ellipse cx="32" cy="52" rx="14" ry="3" fill="rgba(0,0,0,0.25)" />
        <path d="M32 8c10 0 17 16 17 27s-8 17-17 17-17-6-17-17S22 8 32 8z" fill="#fff" stroke="#1d1d1d" strokeWidth="2" />
        <path d="M24 26c3-3 7-2 7 1s-4 5-7 3zM36 38c3-2 7-1 6 2s-5 4-6-2zM37 18c2-1 4 0 3 2s-3 2-3-2z" fill="#1d1d1d" />
        <path d="M22 33l4-3 3 3 4-3 3 3" fill="none" stroke="#c9a46a" strokeWidth="1.4" strokeLinejoin="round" />
      </svg>
    );
  }
  return (
    <svg className={"b-cow st-" + id} viewBox="0 0 64 56" aria-hidden>
      <ellipse cx="32" cy="40" rx="22" ry="13" fill="#fff" stroke="#1d1d1d" strokeWidth="2" />
      <path d="M18 36c3-4 8-4 9 0s-4 7-9 4zM38 44c3-3 8-2 8 2s-6 5-8-2z" fill="#1d1d1d" />
      {id !== "baby" && <path d={id === "young" ? "M16 13l-3-3M48 13l3-3" : "M14 14l-6-6M50 14l6-6"} stroke="#c9a46a" strokeWidth="3" strokeLinecap="round" />}
      <ellipse cx="32" cy="22" rx="16" ry="13" fill="#fff" stroke="#1d1d1d" strokeWidth="2" />
      <ellipse cx="32" cy="29" rx="10" ry="6" fill="#ffb3c7" stroke="#1d1d1d" strokeWidth="1.5" />
      <circle cx="28" cy="29" r="1.4" fill="#1d1d1d" />
      <circle cx="36" cy="29" r="1.4" fill="#1d1d1d" />
      <path d="M17 52v-4M47 52v-4" stroke="#1d1d1d" strokeWidth="3" strokeLinecap="round" />
      {id === "crowned" && <path d="M23 10l-2-9 6 4 5-5 5 5 6-4-2 9z" fill="#ffd24d" stroke="#8a6410" strokeWidth="1.2" strokeLinejoin="round" />}
    </svg>
  );
}

/** Generic widget window for every form that is not island, edge or pop. */
export function Board({ id }: { id: string }) {
  const [cfg] = useDisplay();
  const [pulse, setPulse] = useState<number | null>(null);
  const [feed, setFeed] = useState<Change[]>([]);
  const seq = useRef(0);
  const pulseTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const views = useEntries((e, map) => {
    if (e.type !== "valueChanged") return;
    const v = map.get(e.entryId);
    if (v) setFeed((q) => [{ key: ++seq.current, view: { ...v, value: e.value }, diff: e.value - e.prev, at: Date.now() }, ...q].slice(0, 12));
    setPulse(e.entryId);
    clearTimeout(pulseTimer.current);
    pulseTimer.current = setTimeout(() => setPulse(null), 1400);
  });
  const now = useNow(id === "feed" || id === "pet" ? 10000 : 60000);
  const stages = useRef(new Map<number, number>());
  const [petUp, setPetUp] = useState<PetUp | null>(null);
  useEffect(() => {
    if (id !== "pet") return;
    for (const v of views.values()) {
      const st = petStage(v);
      const was = stages.current.get(v.entry.id);
      stages.current.set(v.entry.id, st);
      if (was !== undefined && st > was) setPetUp({ key: ++seq.current, entryId: v.entry.id, stage: st });
    }
  }, [id, views]);
  useEffect(() => {
    if (!petUp) return;
    const t = setTimeout(() => setPetUp(null), 2000);
    return () => clearTimeout(t);
  }, [petUp?.key]);
  const [open, setOpen] = useState(false);
  const leave = useRef<ReturnType<typeof setTimeout>>(undefined);
  const [idx, setIdx] = useState(0);
  const [peekOn, setPeekOn] = useState(false);
  const [root, setRoot] = useState<HTMLDivElement | null>(null);

  const f = cfg?.forms?.[id];
  const list = f ? selectEntries([...views.values()], f) : [];
  const n = list.length;

  useEffect(() => {
    if (open || n < 2 || !f) return;
    const t = setInterval(() => setIdx((i) => (i + 1) % n), Math.max(2, f.cycleS) * 1000);
    return () => clearInterval(t);
  }, [open, n, f?.cycleS]);

  useEffect(() => {
    if (id !== "peek") return;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const un = listen("peek", () => {
      setPeekOn(true);
      clearTimeout(timer);
      timer = setTimeout(() => setPeekOn(false), PEEK_MS);
    });
    return () => {
      clearTimeout(timer);
      un.then((u) => u());
    };
  }, [id]);

  const visible = !!f?.enabled && (id !== "peek" || peekOn);
  useEffect(() => {
    if (f && !visible) api.widgetDone().catch(() => {});
  }, [visible, !!f]);
  useWidgetSize(visible ? root : null, visible);

  if (!cfg || !f || !visible) return null;
  const s = formScale(f);
  const look = formLook(f);
  const cur = n ? list[idx % n] : null;
  const c: Ctx = {
    f,
    s,
    list,
    cur,
    idx: n ? idx % n : 0,
    open,
    live: true,
    leave: () => setOpen(false),
    pulse,
    feed,
    now,
    trip: Math.round((window.innerWidth + 320 * s) / 70),
    petUp,
    num: (v, h, compact) => <Skin skin={f.skin} tint={f.tint} opts={f.skinOpts} text={valueText(v, compact)} height={h * 1.15} status={v.status} />,
    icon: (v, size) => (f.showIcons ? <EntryIcon icon={v.entry.icon} size={size} /> : null),
  };
  const layout = LAYOUTS[id];
  const empty = n === 0 && id !== "feed";

  return (
    <div className={`widget-root board-root form-${id}` + edgeOf(id, f) + look.className + formOptClasses(id, f) + (id === "train" && formOpt(id, f, "travel") ? " travel" : "") + (id === "drawer" ? " dock-" + f.side : "")} ref={setRoot} style={look.style}>
      <ChangeFx f={f} views={list} />
      <div
        className="b-host"
        onMouseEnter={() => {
          if (f.hoverExpands === false) return;
          clearTimeout(leave.current);
          setOpen(true);
        }}
        onMouseLeave={() => {
          leave.current = setTimeout(() => setOpen(false), 350);
        }}
        onClick={() => {
          if (id === "peek") setPeekOn(false);
          else if (id === "drawer") setOpen((o) => !o);
          else if (f.clickOpens !== false) api.openSettings();
        }}
      >
        {empty ? <div className="b-panel b-empty">Cownter · add a number</div> : layout ? layout(c) : null}
      </div>
    </div>
  );
}

/** Static render of a form with the given numbers, used for gallery thumbnails and the customize preview. */
export function FormPreview({ id, f, views }: { id: string; f: FormCfg; views: EntryView[] }) {
  const picked = selectEntries(views, f);
  const list = picked.length ? picked : views;
  const s = formScale(f);
  const look = formLook(f);
  const cur = list[0] ?? null;
  const num = (v: EntryView, h: number, compact?: boolean) => <Skin skin={f.skin} tint={f.tint} opts={f.skinOpts} text={valueText(v, compact)} height={h * 1.15} status={v.status} />;
  const icon = (v: EntryView, size: number) => (f.showIcons ? <EntryIcon icon={v.entry.icon} size={size} /> : null);
  const feed: Change[] = list.slice(0, 3).map((v, i) => ({ key: i, view: v, diff: v.deltaToday || 1, at: Date.now() - (i + 1) * 60000 }));
  const c: Ctx = { f, s, list, cur, idx: 0, open: false, live: false, pulse: null, feed, now: Date.now(), trip: 0, num, icon };

  let body: React.ReactNode;
  if (id === "island") {
    body = (
      <div className="island">
        {cur && (
          <div className="island-row">
            {icon(cur, 18 * s)}
            {num(cur, 19 / 1.15 * s)}
            <Delta v={cur} />
            <Dots c={c} />
          </div>
        )}
      </div>
    );
  } else if (id === "edge") {
    body = (
      <div className={"edge-root side-" + f.side}>
        <div className="edge">
          {list.map((v) => (
            <div className="edge-row" key={v.entry.id}>
              {icon(v, 18 * s)}
              {num(v, 12 / 1.15 * s, true)}
            </div>
          ))}
        </div>
      </div>
    );
  } else if (id === "pop") {
    body = cur && (
      <div className="pop pop-static">
        {icon(cur, 24 * s)}
        <div className="pop-body">
          <span className="pop-title">
            <b className="up">+12</b> {cur.entry.name}
          </span>
          {num(cur, 21 / 1.15 * s)}
        </div>
      </div>
    );
  } else {
    const layout = LAYOUTS[id];
    body = list.length || id === "feed" ? layout?.(c) : null;
  }
  return (
    <div className={`widget-root board-root form-${id}` + edgeOf(id, f) + look.className + formOptClasses(id, f)} style={look.style}>
      <div className="b-host">{body}</div>
    </div>
  );
}
