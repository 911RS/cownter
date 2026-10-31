import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { api } from "../core/api";
import { selectEntries } from "../core/select";
import type {
  DisplayConfig,
  EntryView,
  FormCfg,
  Group,
  Position,
} from "../core/types";
import { FormPreview } from "../forms/Board";
import { formOpt, formOptsFor } from "../forms/options";
import { BOARD_IDS, FORM_INFO } from "../forms/registry";
import { SKINS, Skin, TINTS, type SkinGroup } from "../skins/Skin";
import { SKIN_OPTS, skinOpt } from "../skins/options";
import { Group as Section, prettyKey, Row, Segmented, Toggle } from "./ui";

const DOCKED = ["edge", "drawer"];
const CYCLING = ["island", "orb", "notch", "tag", "pet", "pager"];
const DOTTED = ["island", "notch", "tag", "pager"];
const NAMED = BOARD_IDS.filter((id) => id !== "train" && id !== "pet");
const NO_ICONS = ["pager"];
const NO_DELTA = ["trend", "train", "feed"];
const HOVERS = ["island", "edge", "orb", "drawer", "notch", ...CYCLING];

/** Shape and spot of the little widget drawn on each gallery tile. */
const MINI_SHAPE: Record<string, string> = {
  island: "pill",
  bar: "bar",
  pills: "pills",
  orb: "orb",
  ticker: "bar",
  trend: "card",
  chips: "notes",
  versus: "bar",
  stream: "wide",
  edge: "tab",
  drawer: "tab",
  notch: "notch",
  wings: "wings",
  toptabs: "tabs",
  folders: "tabs",
  deck: "deck",
  tag: "tag",
  clothesline: "line",
  train: "train",
  pet: "pet",
  pager: "card",
  pop: "pill",
  feed: "card",
  peek: "peek",
  tray: "tray",
  confetti: "confetti",
};

function getForm(cfg: DisplayConfig, id: string): FormCfg | undefined {
  if (id === "island" || id === "edge" || id === "pop") return cfg[id];
  return cfg.forms?.[id];
}

function setForm(cfg: DisplayConfig, id: string, f: FormCfg): DisplayConfig {
  if (id === "island" || id === "edge" || id === "pop")
    return { ...cfg, [id]: f };
  return { ...cfg, forms: { ...cfg.forms, [id]: f } };
}

function describe(cfg: DisplayConfig, info: { id: string; text: string }): string {
  if (info.id === "peek") {
    return cfg.hotkeyPeek ? `Press ${prettyKey(cfg.hotkeyPeek)} to see every number in the middle of the screen for a few seconds.` : "Set a quick peek shortcut under General, then press it to see every number for a few seconds.";
  }
  return info.text;
}

function isOn(cfg: DisplayConfig, id: string) {
  if (id === "tray") return cfg.tray.showNumber;
  if (id === "confetti") return cfg.celebration;
  return !!getForm(cfg, id)?.enabled;
}

function Mini({ id, f, on }: { id: string; f?: FormCfg; on: boolean }) {
  const shape = MINI_SHAPE[id] ?? "pill";
  const pos = f?.position ?? "topCenter";
  const docked = DOCKED.includes(id);
  const style: React.CSSProperties = {};
  if (docked) {
    style[f?.side === "left" ? "left" : "right"] = 0;
    style.top = `${15 + (f?.offset ?? 0.4) * 45}%`;
  } else if (
    ![
      "notch",
      "wings",
      "toptabs",
      "deck",
      "tray",
      "confetti",
      "line",
      "peek",
    ].includes(shape) ||
    shape === "peek"
  ) {
    if (pos.endsWith("Left")) style.left = 4;
    else if (pos.endsWith("Right")) style.right = 4;
    else style.left = "50%";
    if (pos.startsWith("bottom")) style.bottom = 4;
    else if (pos.startsWith("middle")) style.top = "50%";
    else style.top = id === "tag" ? 0 : 4;
    style.translate = `${pos.endsWith("Center") ? "-50%" : "0"} ${pos.startsWith("middle") ? "-50%" : "0"}`;
  }
  return (
    <span className={"mini" + (on ? " on" : "")} aria-hidden>
      <i
        className={
          "mini-" + shape + (docked && f?.side === "left" ? " left" : "")
        }
        style={style}
      />
    </span>
  );
}

const POSITIONS: { id: Position; label: string }[] = [
  { id: "topLeft", label: "Top left" },
  { id: "topCenter", label: "Top centre" },
  { id: "topRight", label: "Top right" },
  { id: "middleLeft", label: "Middle left" },
  { id: "middleCenter", label: "Centre" },
  { id: "middleRight", label: "Middle right" },
  { id: "bottomLeft", label: "Bottom left" },
  { id: "bottomCenter", label: "Bottom centre" },
  { id: "bottomRight", label: "Bottom right" },
];

const LOOK_DEFAULTS: Partial<FormCfg> = {
  size: "m",
  scale: 1,
  opacity: 1,
  bg: "#000000",
  bgAlpha: 1,
  roundness: 1,
  spacing: 1,
  gap: 26,
  shadow: true,
};

function Slider({
  value,
  min,
  max,
  step = 1,
  onChange,
  unit = "",
  fine = false,
}: {
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (v: number) => void;
  unit?: string;
  fine?: boolean;
}) {
  const clamp = (v: number) => Math.min(max, Math.max(min, Math.round(v)));
  if (fine) {
    const nudge = (d: number) => (e: React.MouseEvent) => onChange(clamp(value + d * (e.shiftKey ? 10 : 1)));
    return (
      <div className="slider fine">
        <button type="button" className="nudge" aria-label="Less" title="1 px (Shift: 10 px)" onClick={nudge(-1)}>
          −
        </button>
        <input
          type="range"
          min={min}
          max={max}
          step={step}
          value={value}
          onChange={(e) => onChange(Number(e.target.value))}
        />
        <button type="button" className="nudge" aria-label="More" title="1 px (Shift: 10 px)" onClick={nudge(1)}>
          +
        </button>
        <input
          className="slider-num"
          type="number"
          min={min}
          max={max}
          value={value}
          aria-label="Exact value"
          onChange={(e) => e.target.value !== "" && Number.isFinite(Number(e.target.value)) && onChange(clamp(Number(e.target.value)))}
          onKeyDown={(e) => {
            if (e.key === "ArrowUp" || e.key === "ArrowDown") {
              e.preventDefault();
              onChange(clamp(value + (e.key === "ArrowUp" ? 1 : -1) * (e.shiftKey ? 10 : 1)));
            }
          }}
        />
        <span className="slider-unit">{unit.trim()}</span>
      </div>
    );
  }
  return (
    <div className="slider">
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
      />
      <span className="slider-val">
        {value}
        {unit}
      </span>
    </div>
  );
}

function FormSettings({
  k,
  f,
  views,
  previewViews,
  groups,
  monitors,
  onChange,
}: {
  k: string;
  f: FormCfg;
  views: EntryView[];
  previewViews: EntryView[];
  groups: Group[];
  monitors: string[];
  onChange: (f: FormCfg) => void;
}) {
  const [sub, setSub] = useState<"font" | "content" | "position" | "look">(
    "font",
  );
  const shown = selectEntries(views, f);
  const showValue = f.entryIds.length
    ? "pick"
    : f.groupId !== null
      ? `g${f.groupId}`
      : "all";

  return (
    <>
      <div
        className={
          "stage" +
          (f.tint === "classic" &&
          (f.skin === "dotMatrix" || f.skin === "calculator")
            ? " classic"
            : "")
        }
      >
        <FormPreview id={k} f={f} views={previewViews} />
      </div>
      <div className="subtabs">
        <Segmented
          value={sub}
          onChange={setSub}
          options={[
            { id: "font", label: "Font" },
            { id: "content", label: "Content" },
            { id: "position", label: "Position" },
            { id: "look", label: "Look" },
          ]}
        />
      </div>

      {sub === "font" && <LookPicker f={f} onChange={onChange} />}

      {sub === "content" && (
        <>
          <Row
            label="Shows"
            hint={`${shown.length} of ${views.length} numbers`}
          >
            <select
              value={showValue}
              onChange={(e) => {
                const v = e.target.value;
                if (v === "all")
                  onChange({ ...f, groupId: null, entryIds: [] });
                else if (v === "pick")
                  onChange({
                    ...f,
                    groupId: null,
                    entryIds: views.slice(0, 1).map((x) => x.entry.id),
                  });
                else
                  onChange({ ...f, groupId: Number(v.slice(1)), entryIds: [] });
              }}
            >
              <option value="all">All numbers</option>
              {groups.map((g) => (
                <option key={g.id} value={`g${g.id}`}>
                  Group: {g.name}
                </option>
              ))}
              <option value="pick">Choose numbers…</option>
            </select>
          </Row>
          {showValue === "pick" && (
            <div className="chips">
              {views.map((v) => {
                const on = f.entryIds.includes(v.entry.id);
                return (
                  <button
                    type="button"
                    key={v.entry.id}
                    className={"pill-btn" + (on ? " on" : "")}
                    aria-pressed={on}
                    onClick={() => {
                      const ids = on
                        ? f.entryIds.filter((x) => x !== v.entry.id)
                        : [...f.entryIds, v.entry.id];
                      onChange({ ...f, entryIds: ids });
                    }}
                  >
                    {on ? "✓ " : ""}
                    {v.entry.name}
                  </button>
                );
              })}
            </div>
          )}
          {!NO_ICONS.includes(k) && (
            <Row label="Icons">
              <Toggle
                checked={f.showIcons}
                onChange={(showIcons) => onChange({ ...f, showIcons })}
              />
            </Row>
          )}
          {!NO_ICONS.includes(k) && f.showIcons && (
            <Row label="Icon style" hint="Tile, a bigger tile, a colour fade from the left, a glowing badge, a bare glyph, or a colour bar on the left.">
              <Segmented
                value={f.iconStyle ?? "tile"}
                onChange={(iconStyle) => onChange({ ...f, iconStyle })}
                options={[
                  { id: "tile", label: "Tile" },
                  { id: "big", label: "Big" },
                  { id: "fade", label: "Fade" },
                  { id: "glow", label: "Glow" },
                  { id: "glyph", label: "Glyph" },
                  { id: "bar", label: "Bar" },
                ]}
              />
            </Row>
          )}
          {!NO_DELTA.includes(k) && (
            <Row
              label="Today's change"
              hint="The green or red difference next to the number."
            >
              <Toggle
                checked={f.showDelta}
                onChange={(showDelta) => onChange({ ...f, showDelta })}
              />
            </Row>
          )}
          {NAMED.includes(k) && (
            <Row label="Names">
              <Toggle
                checked={f.showNames}
                onChange={(showNames) => onChange({ ...f, showNames })}
              />
            </Row>
          )}
          {DOTTED.includes(k) && (
            <Row
              label="Page dots"
              hint="The little dots that show which number is on screen."
            >
              <Toggle
                checked={f.showDots}
                onChange={(showDots) => onChange({ ...f, showDots })}
              />
            </Row>
          )}
          {k !== "pop" &&
            k !== "peek" &&
            !(k === "train" && formOpt(k, f, "travel")) && (
              <>
                <Row
                  label="Click"
                  hint="What happens when you click the widget."
                >
                  <Segmented
                    value={f.clickOpens ? "open" : "none"}
                    onChange={(v) =>
                      onChange({ ...f, clickOpens: v === "open" })
                    }
                    options={[
                      { id: "open", label: "Open Cownter" },
                      { id: "none", label: "Nothing" },
                    ]}
                  />
                </Row>
                {HOVERS.includes(k) && (
                  <Row
                    label="Hover"
                    hint="What happens when the mouse is over it."
                  >
                    <Segmented
                      value={f.hoverExpands ? "expand" : "none"}
                      onChange={(v) =>
                        onChange({ ...f, hoverExpands: v === "expand" })
                      }
                      options={[
                        {
                          id: "expand",
                          label:
                            CYCLING.includes(k) &&
                            !["island", "orb", "notch"].includes(k)
                              ? "Pause"
                              : "Show details",
                        },
                        { id: "none", label: "Nothing" },
                      ]}
                    />
                  </Row>
                )}
              </>
            )}
          {CYCLING.includes(k) && (
            <Row label="Switch number every">
              <Slider
                value={f.cycleS}
                min={2}
                max={60}
                unit=" s"
                onChange={(cycleS) => onChange({ ...f, cycleS })}
              />
            </Row>
          )}
          <div className="row-head">Show on the widget</div>
          {formOptsFor(k).map((o) => (
            <Row key={o.id} label={o.label}>
              <Toggle
                checked={formOpt(k, f, o.id)}
                ariaLabel={o.label}
                onChange={(v) =>
                  onChange({
                    ...f,
                    formOpts: { ...(f.formOpts ?? {}), [o.id]: v },
                  })
                }
              />
            </Row>
          ))}
        </>
      )}

      {sub === "position" && (
        <>
          {monitors.length > 1 && (
            <Row label="Screen">
              <select
                value={f.monitor}
                onChange={(e) => onChange({ ...f, monitor: e.target.value })}
              >
                <option value="">Main screen</option>
                {monitors.map((m) => (
                  <option key={m} value={m}>
                    {m}
                  </option>
                ))}
              </select>
            </Row>
          )}
          {DOCKED.includes(k) ? (
            <>
              <Row label="Side">
                <Segmented
                  value={f.side}
                  onChange={(side) => onChange({ ...f, side })}
                  options={[
                    { id: "left", label: "Left" },
                    { id: "right", label: "Right" },
                  ]}
                />
              </Row>
              <Row label="Height on screen">
                <Slider
                  value={Math.round(f.offset * 100)}
                  min={0}
                  max={100}
                  unit="%"
                  onChange={(v) => onChange({ ...f, offset: v / 100 })}
                />
              </Row>
            </>
          ) : (
            <Row
              label="Anchor"
              hint={POSITIONS.find((p) => p.id === f.position)?.label}
            >
              <div className="anchor-grid" role="radiogroup">
                {POSITIONS.map((p) => (
                  <button
                    type="button"
                    role="radio"
                    aria-checked={f.position === p.id}
                    aria-label={p.label}
                    title={p.label}
                    key={p.id}
                    className={f.position === p.id ? "on" : ""}
                    onClick={() => onChange({ ...f, position: p.id })}
                  />
                ))}
              </div>
            </Row>
          )}
          <Row
            label="Move sideways"
            hint={
              DOCKED.includes(k)
                ? "Distance from the screen edge."
                : "Positive moves right."
            }
          >
            <Slider
              value={f.offsetX}
              min={DOCKED.includes(k) ? -40 : -1600}
              max={1600}
              unit=" px"
              fine
              onChange={(offsetX) => onChange({ ...f, offsetX })}
            />
          </Row>
          <Row label="Move up or down" hint="Positive moves down.">
            <Slider
              value={f.offsetY}
              min={-1000}
              max={1000}
              unit=" px"
              fine
              onChange={(offsetY) => onChange({ ...f, offsetY })}
            />
          </Row>
          <Row label="Reset position">
            <button
              type="button"
              className="btn ghost"
              onClick={() =>
                onChange({
                  ...f,
                  offsetX: 0,
                  offsetY: k === "island" ? 8 : 0,
                  monitor: "",
                })
              }
            >
              Reset
            </button>
          </Row>
        </>
      )}

      {sub === "look" && (
        <>
          <Row label="Size">
            <Segmented
              value={f.size}
              onChange={(size) => onChange({ ...f, size })}
              options={[
                { id: "xs", label: "XS" },
                { id: "s", label: "S" },
                { id: "m", label: "M" },
                { id: "l", label: "L" },
                { id: "xl", label: "XL" },
              ]}
            />
          </Row>
          <Row label="Fine size">
            <Slider
              value={Math.round(f.scale * 100)}
              min={60}
              max={200}
              step={5}
              unit="%"
              onChange={(v) => onChange({ ...f, scale: v / 100 })}
            />
          </Row>
          <Row label="Background">
            <input
              type="color"
              value={f.bg}
              onChange={(e) => onChange({ ...f, bg: e.target.value })}
              aria-label="Background colour"
            />
          </Row>
          <Row label="Background opacity">
            <Slider
              value={Math.round(f.bgAlpha * 100)}
              min={0}
              max={100}
              step={5}
              unit="%"
              onChange={(v) => onChange({ ...f, bgAlpha: v / 100 })}
            />
          </Row>
          <Row label="Corner roundness">
            <Slider
              value={Math.round(f.roundness * 100)}
              min={0}
              max={100}
              step={5}
              unit="%"
              onChange={(v) => onChange({ ...f, roundness: v / 100 })}
            />
          </Row>
          {!["island", "edge", "pop", "orb", "pet", "peek"].includes(k) && (
            <Row label="Spacing" hint="Space between the tiles and items.">
              <Slider
                value={Math.round((f.spacing ?? 1) * 100)}
                min={0}
                max={300}
                step={10}
                unit="%"
                onChange={(v) => onChange({ ...f, spacing: v / 100 })}
              />
            </Row>
          )}
          {k === "wings" && (
            <Row label="Middle gap" hint="Room between the two wings, for a camera notch or a clock.">
              <Slider
                value={f.gap ?? 26}
                min={0}
                max={600}
                step={2}
                unit=" px"
                fine
                onChange={(gap) => onChange({ ...f, gap })}
              />
            </Row>
          )}
          <Row label="Widget opacity">
            <Slider
              value={Math.round(f.opacity * 100)}
              min={20}
              max={100}
              step={5}
              unit="%"
              onChange={(v) => onChange({ ...f, opacity: v / 100 })}
            />
          </Row>
          <Row label="Shadow">
            <Toggle
              checked={f.shadow}
              onChange={(shadow) => onChange({ ...f, shadow })}
            />
          </Row>
          <Row label="Reset look">
            <button
              type="button"
              className="btn ghost"
              onClick={() => onChange({ ...f, ...LOOK_DEFAULTS })}
            >
              Reset
            </button>
          </Row>
        </>
      )}
    </>
  );
}

const EXTRAS = ["pop", "peek", "tray", "confetti"];
const MAIN = FORM_INFO.filter((t) => !EXTRAS.includes(t.id)).map((t) => t.id);

function sampleViews(): EntryView[] {
  const now = Date.now();
  const mk = (
    id: number,
    name: string,
    value: number,
    delta: number,
    icon: EntryView["entry"]["icon"],
  ): EntryView => ({
    entry: {
      id,
      slug: `s${id}`,
      name,
      icon,
      groupId: null,
      source: { type: "formula", expr: "1" },
      intervalS: 300,
      format: { style: "full", decimals: null, prefix: "", suffix: "" },
      goalValue: null,
      celebrate: false,
      sounds: { onChange: null, onGoal: null },
      sort: id,
    },
    value,
    prev: value - delta,
    status: "live",
    error: null,
    updatedAt: now,
    deltaToday: delta,
    spark: Array.from(
      { length: 12 },
      (_, i) =>
        [
          now - (12 - i) * 3600e3,
          value - delta + Math.round(Math.sin(i) * delta) + i * 3,
        ] as [number, number],
    ),
    goalProgress: null,
  });
  return [
    mk(-1, "Subscribers", 48213, 124, {
      kind: "builtin",
      value: "play",
      bg: "#e5484d",
    }),
    mk(-2, "Stars", 1932, 12, {
      kind: "builtin",
      value: "star",
      bg: "#24292f",
    }),
    mk(-3, "Orders", 87, -3, { kind: "builtin", value: "cart", bg: "#12805c" }),
  ];
}

function Thumb({
  id,
  f,
  views,
  max = 0.62,
  big = false,
}: {
  id: string;
  f?: FormCfg;
  views: EntryView[];
  max?: number;
  big?: boolean;
}) {
  const box = useRef<HTMLSpanElement>(null);
  const inner = useRef<HTMLSpanElement>(null);
  const [scale, setScale] = useState(0.5);
  useLayoutEffect(() => {
    const b = box.current;
    const c = inner.current;
    if (!b || !c) return;
    const fit = () => {
      const bw = b.clientWidth - 12;
      const bh = b.clientHeight - 12;
      const cw = c.offsetWidth || 1;
      const ch = c.offsetHeight || 1;
      setScale(Math.min(max, bw / cw, bh / ch));
    };
    const ro = new ResizeObserver(fit);
    ro.observe(b);
    ro.observe(c);
    fit();
    return () => ro.disconnect();
  }, [max, id]);
  if (!f || id === "tray" || id === "confetti")
    return <Mini id={id} f={f} on />;
  const pos = DOCKED.includes(id)
    ? f.side === "left"
      ? "middleLeft"
      : "middleRight"
    : f.position;
  const x = pos.endsWith("Left") ? 0 : pos.endsWith("Right") ? 1 : 0.5;
  const y = pos.startsWith("bottom") ? 1 : pos.startsWith("middle") ? 0.5 : 0;
  const flush =
    DOCKED.includes(id) ||
    [
      "notch",
      "wings",
      "toptabs",
      "folders",
      "tag",
      "clothesline",
      "deck",
      "island",
    ].includes(id);
  const m = flush ? 0 : big ? 16 : 6;
  return (
    <span className={big ? "thumb big" : "thumb"} ref={box} aria-hidden>
      <span
        className="thumb-scale"
        ref={inner}
        style={{
          left: `calc(${x * 100}% + ${m * (1 - 2 * x)}px)`,
          top: `calc(${y * 100}% + ${m * (1 - 2 * y)}px)`,
          transform: `translate(${-x * 100}%, ${-y * 100}%) scale(${scale})`,
          transformOrigin: `${x * 100}% ${y * 100}%`,
        }}
      >
        <FormPreview
          id={id}
          f={{
            ...f,
            enabled: true,
            offsetX: 0,
            offsetY: 0,
            ...(big ? {} : { scale: 1, size: "m" }),
          }}
          views={views.slice(0, big ? 4 : 3)}
        />
      </span>
    </span>
  );
}

export function DisplayPage({
  cfg,
  views,
  groups,
  onChange,
}: {
  cfg: DisplayConfig;
  views: EntryView[];
  groups: Group[];
  onChange: (c: DisplayConfig) => void;
}) {
  const [editing, setEditing] = useState<string | null>(null);
  const [browse, setBrowse] = useState<string | null>(null);
  const [cat, setCat] = useState("All");
  const [monitors, setMonitors] = useState<string[]>([]);
  useEffect(() => {
    api
      .listMonitors()
      .then((m) => setMonitors(Array.isArray(m) ? m : []))
      .catch(() => {});
  }, []);
  const shown = views.some((v) => v.value !== null) ? views : sampleViews();

  const choose = (id: string) => {
    let next = cfg;
    const from = MAIN.map((m) => getForm(cfg, m)).find((f) => f?.enabled);
    for (const m of MAIN) {
      const f = getForm(next, m);
      if (!f) continue;
      if (m === id) {
        const look = from && from !== f ? { skin: from.skin, tint: from.tint, skinOpts: from.skinOpts } : {};
        next = setForm(next, m, { ...f, ...look, enabled: true });
      } else if (f.enabled) next = setForm(next, m, { ...f, enabled: false });
    }
    onChange(next);
  };
  const lookOf = (id: string): FormCfg | undefined => {
    const f = getForm(cfg, id);
    const cur = MAIN.map((m) => getForm(cfg, m)).find((x) => x?.enabled);
    return f && cur && !f.enabled ? { ...f, skin: cur.skin, tint: cur.tint, skinOpts: cur.skinOpts } : f;
  };
  const toggleExtra = (id: string, v: boolean) => {
    if (id === "tray")
      onChange({ ...cfg, tray: { ...cfg.tray, showNumber: v } });
    else if (id === "confetti") onChange({ ...cfg, celebration: v });
    else {
      const f = getForm(cfg, id);
      if (f) onChange(setForm(cfg, id, { ...f, enabled: v }));
    }
  };

  if (editing) {
    const info = FORM_INFO.find((t) => t.id === editing)!;
    const form = getForm(cfg, editing);
    return (
      <div className="page">
        <header className="page-head" data-tauri-drag-region>
          <div className="row">
            <button
              type="button"
              className="back-btn"
              onClick={() => setEditing(null)}
            >
              <svg
                viewBox="0 0 24 24"
                width="16"
                height="16"
                fill="none"
                stroke="currentColor"
                strokeWidth={2.2}
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M15 5l-7 7 7 7" />
              </svg>
              Styles
            </button>
            <h1>{info.title}</h1>
          </div>
        </header>
        <section className="group">
          <div className="group-body">
            <Row label={describe(cfg, info)} />
            {form && (
              <FormSettings
                key={editing}
                k={editing}
                f={form}
                views={views}
                previewViews={shown}
                groups={groups}
                monitors={monitors}
                onChange={(f) => onChange(setForm(cfg, editing, f))}
              />
            )}
            {editing === "peek" && (
              <Row label="Shortcut" hint="Change it on the General page." />
            )}
            {editing === "tray" && (
              <Row label="Number" hint="On Windows it is drawn into the icon.">
                <select
                  value={cfg.tray.entryId ?? ""}
                  onChange={(e) =>
                    onChange({
                      ...cfg,
                      tray: {
                        ...cfg.tray,
                        entryId: e.target.value ? Number(e.target.value) : null,
                      },
                    })
                  }
                >
                  <option value="">None</option>
                  {views.map((v) => (
                    <option key={v.entry.id} value={v.entry.id}>
                      {v.entry.name}
                    </option>
                  ))}
                </select>
              </Row>
            )}
          </div>
        </section>
      </div>
    );
  }

  const active = MAIN.find((id) => getForm(cfg, id)?.enabled) ?? null;
  const cats = [
    "All",
    ...new Set(
      FORM_INFO.filter((t) => MAIN.includes(t.id)).map((t) => t.group),
    ),
  ];
  const strip = FORM_INFO.filter(
    (t) => MAIN.includes(t.id) && (cat === "All" || t.group === cat),
  );
  const viewing =
    FORM_INFO.find((t) => t.id === (browse ?? active)) ?? strip[0];
  const vIdx = strip.findIndex((t) => t.id === viewing.id);
  const step = (d: number) => {
    const list = strip.length ? strip : FORM_INFO;
    const next = list[(Math.max(0, vIdx) + d + list.length) % list.length];
    setBrowse(next.id);
    choose(next.id);
    document
      .getElementById("strip-" + next.id)
      ?.scrollIntoView({
        block: "nearest",
        inline: "center",
        behavior: "smooth",
      });
  };
  const inUse = viewing.id === active;

  return (
    <div className="page">
      <header className="page-head" data-tauri-drag-region>
        <h1>On screen</h1>
      </header>

      <div className="cats" role="tablist">
        {cats.map((c) => (
          <button
            type="button"
            role="tab"
            aria-selected={cat === c}
            key={c}
            className={"pill-btn" + (cat === c ? " on" : "")}
            onClick={() => setCat(c)}
          >
            {c}
          </button>
        ))}
      </div>

      <div
        className="showroom"
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === "ArrowRight") step(1);
          if (e.key === "ArrowLeft") step(-1);
          if (e.key === "Enter" && !inUse) choose(viewing.id);
        }}
      >
        <Thumb
          key={viewing.id}
          id={viewing.id}
          f={lookOf(viewing.id)}
          views={shown}
          max={1}
          big
        />
        <div className="showroom-bar">
          <div className="arrows">
            <button
              type="button"
              className="icon-btn"
              aria-label="Previous style"
              onClick={() => step(-1)}
            >
              <svg
                viewBox="0 0 24 24"
                width="18"
                height="18"
                fill="none"
                stroke="currentColor"
                strokeWidth={2.2}
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M15 5l-7 7 7 7" />
              </svg>
            </button>
            <button
              type="button"
              className="icon-btn"
              aria-label="Next style"
              onClick={() => step(1)}
            >
              <svg
                viewBox="0 0 24 24"
                width="18"
                height="18"
                fill="none"
                stroke="currentColor"
                strokeWidth={2.2}
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M9 5l7 7-7 7" />
              </svg>
            </button>
          </div>
          <div className="row-text">
            <span className="row-label">
              {viewing.title}
              {inUse && (
                <span className="in-use" title="In use" aria-label="In use">
                  <svg
                    viewBox="0 0 24 24"
                    width="11"
                    height="11"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth={3.2}
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M5 12.5l4.5 4.5L19 7.5" />
                  </svg>
                </span>
              )}
            </span>
            <span className="row-hint">{describe(cfg, viewing)}</span>
          </div>
          <button
            type="button"
            className="btn primary"
            onClick={() =>
              inUse ? setEditing(viewing.id) : choose(viewing.id)
            }
          >
            {inUse ? "Customize" : "Use this style"}
          </button>
        </div>
      </div>

      <div className="strip" role="listbox" aria-label="Styles">
        {strip.map((t) => (
          <button
            type="button"
            role="option"
            aria-selected={t.id === viewing.id}
            id={"strip-" + t.id}
            key={t.id}
            className={
              "strip-item" +
              (t.id === viewing.id ? " sel" : "") +
              (t.id === active ? " active" : "")
            }
            onClick={() => {
              setBrowse(t.id);
              if (t.id !== active) choose(t.id);
            }}
          >
            <Thumb id={t.id} f={lookOf(t.id)} views={shown} />
            <span className="strip-name">{t.title}</span>
          </button>
        ))}
      </div>

      <Section title="Add-ons">
        {EXTRAS.map((id) => {
          const info = FORM_INFO.find((t) => t.id === id)!;
          const on = isOn(cfg, id);
          return (
            <Row key={id} label={info.title} hint={describe(cfg, info)}>
              {on && id !== "confetti" && (
                <button
                  type="button"
                  className="link"
                  onClick={() => setEditing(id)}
                >
                  Customize
                </button>
              )}
              <Toggle checked={on} onChange={(v) => toggleExtra(id, v)} />
            </Row>
          );
        })}
      </Section>
    </div>
  );
}

const LOOK_GROUPS: ("All" | SkinGroup)[] = [
  "All",
  "Sans",
  "Condensed",
  "Wide",
  "Mono",
  "Retro",
];

function cardBg(f: FormCfg): string {
  const n = parseInt(f.bg.slice(1), 16);
  const a = Math.max(f.bgAlpha, 0.85);
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${a})`;
}

function LookPicker({
  f,
  onChange,
}: {
  f: FormCfg;
  onChange: (f: FormCfg) => void;
}) {
  const current = SKINS.find((x) => x.id === f.skin) ?? SKINS[0];
  const [cat, setCat] = useState<"All" | SkinGroup>("All");
  const list = SKINS.filter((x) => cat === "All" || x.group === cat);
  const bg = cardBg(f);
  const opts = SKIN_OPTS[current.id] ?? [];
  return (
    <>
      <div className="look-cats" role="tablist" aria-label="Font categories">
        {LOOK_GROUPS.map((g) => (
          <button
            type="button"
            role="tab"
            aria-selected={cat === g}
            key={g}
            className={"look-cat" + (cat === g ? " on" : "")}
            onClick={() => setCat(g)}
          >
            {g}
            <span>
              {g === "All"
                ? SKINS.length
                : SKINS.filter((x) => x.group === g).length}
            </span>
          </button>
        ))}
      </div>
      <div className="look-grid" role="radiogroup" aria-label="Font">
        {list.map((sk) => {
          const on = f.skin === sk.id;
          return (
            <button
              type="button"
              role="radio"
              aria-checked={on}
              key={sk.id}
              className={"look-card" + (on ? " on" : "")}
              onClick={() => onChange({ ...f, skin: sk.id })}
            >
              <span className="look-sample" style={{ background: bg }}>
                <Skin
                  skin={sk.id}
                  tint={f.tint}
                  opts={f.skinOpts}
                  text="48,213"
                  height={24}
                />
              </span>
              <span className="look-meta">
                <b>{sk.label}</b>
                <small>{sk.group}</small>
                {on && (
                  <svg className="look-check" viewBox="0 0 16 16" aria-hidden>
                    <circle cx="8" cy="8" r="8" />
                    <path d="M4.5 8.2l2.3 2.3 4.7-4.9" />
                  </svg>
                )}
              </span>
            </button>
          );
        })}
      </div>
      <Row label="Colour" hint={TINTS.find((t) => t.id === f.tint)?.label}>
        <div className="swatches" role="radiogroup">
          {TINTS.map((t) => (
            <button
              type="button"
              role="radio"
              aria-checked={f.tint === t.id}
              aria-label={t.label}
              title={t.label}
              key={t.id}
              className={"swatch" + (f.tint === t.id ? " on" : "")}
              style={{ background: t.swatch }}
              onClick={() => onChange({ ...f, tint: t.id })}
            />
          ))}
        </div>
      </Row>
      {opts.map((o) => (
        <Row
          key={o.id}
          label={o.label}
          hint={o.id === opts[0].id ? `${current.label} options` : undefined}
        >
          <Toggle
            checked={skinOpt(current.id, f.skinOpts, o.id)}
            ariaLabel={o.label}
            onChange={(v) =>
              onChange({
                ...f,
                skinOpts: {
                  ...(f.skinOpts ?? {}),
                  [`${current.id}.${o.id}`]: v,
                },
              })
            }
          />
        </Row>
      ))}
    </>
  );
}
