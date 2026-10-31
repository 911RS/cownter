import { useEffect, useState } from "react";
import { api, errorText } from "../core/api";
import { formatValue } from "../core/format";
import type { Alert, AlertEffect, AlertKind, EntryView } from "../core/types";
import { FxItem, WIDGET_FX } from "../forms/ChangeFx";
import { NumberInput, Toggle } from "./ui";

const WINDOWS = [
  { s: 900, label: "15 min" },
  { s: 3600, label: "1 hour" },
  { s: 21600, label: "6 hours" },
  { s: 86400, label: "1 day" },
  { s: 604800, label: "1 week" },
];

const KINDS: { id: AlertKind; label: string; def: number }[] = [
  { id: "riseBy", label: "goes up by", def: 1 },
  { id: "dropBy", label: "goes down by", def: 1 },
  { id: "above", label: "goes above", def: 100 },
  { id: "below", label: "goes below", def: 100 },
  { id: "changePct", label: "changes by %", def: 10 },
];

const EFFECTS: { id: AlertEffect; label: string; icon: string }[] = [
  { id: "", label: "None", icon: "∅" },
  { id: "float", label: "Float up", icon: "↑" },
  { id: "pop", label: "Bubble", icon: "◉" },
  { id: "burst", label: "Burst", icon: "✺" },
  { id: "confetti", label: "Confetti", icon: "✦" },
  { id: "rain", label: "Rain", icon: "⁘" },
  { id: "zoom", label: "Zoom", icon: "⤢" },
  { id: "shake", label: "Shake", icon: "≋" },
  { id: "glow", label: "Glow", icon: "☀" },
  { id: "flash", label: "Flash", icon: "ϟ" },
];

const TEMPLATES: { label: string; alert: Pick<Alert, "kind" | "threshold" | "notify" | "effect"> }[] = [
  { label: "+1 on every new one", alert: { kind: "riseBy", threshold: 1, notify: false, effect: "float" } },
  { label: "Celebrate +100", alert: { kind: "riseBy", threshold: 100, notify: true, effect: "confetti" } },
  { label: "Warn on a drop", alert: { kind: "dropBy", threshold: 1, notify: false, effect: "shake" } },
  { label: "Big swing", alert: { kind: "changePct", threshold: 10, notify: true, effect: "flash" } },
];

function sample(a: Pick<Alert, "kind" | "threshold">): { text: string; up: boolean } {
  if (a.kind === "riseBy") return { text: `+${a.threshold}`, up: true };
  if (a.kind === "dropBy") return { text: `-${a.threshold}`, up: false };
  if (a.kind === "below") return { text: "-1", up: false };
  if (a.kind === "changePct") return { text: `+${a.threshold}%`, up: true };
  return { text: "+1", up: true };
}

/** A small fake widget that plays an effect, so the user sees it before saving. */
function FxPreview({ fx, text, up, play }: { fx: AlertEffect; text: string; up: boolean; play: number }) {
  const widget = WIDGET_FX.includes(fx) && play > 0 ? ` fx-${fx}-on ${up ? "fx-up" : "fx-down"}` : "";
  return (
    <span className={"fx-demo" + widget} key={`${play}-${fx}`}>
      <span className="fx-demo-pill">
        <i /> 1,240
      </span>
      {play > 0 && <FxItem fx={fx} text={text} up={up} />}
    </span>
  );
}

function checks(a: Alert, all: Alert[], view?: EntryView): string[] {
  const out: string[] = [];
  if (!a.notify && !a.effect) out.push("This alert does nothing yet. Pick an effect or turn on the notification.");
  if (all.some((x) => x.id !== a.id && x.kind === a.kind && x.threshold === a.threshold && x.windowS === a.windowS)) out.push("Another alert has the same rule.");
  const v = view?.value;
  if (v !== null && v !== undefined && view) {
    if (a.kind === "above" && v > a.threshold) out.push(`It is already above (${formatValue(v, view.entry.format)}), so it fires after it drops below and rises again.`);
    if (a.kind === "below" && v < a.threshold) out.push(`It is already below (${formatValue(v, view.entry.format)}), so it fires after it rises above and drops again.`);
  }
  if ((a.kind === "riseBy" || a.kind === "dropBy") && a.threshold > 1) out.push(`Fires when a single refresh brings a change of ${a.threshold} or more.`);
  return out;
}

export function AlertsEditor({ entryId, view }: { entryId: number; view?: EntryView }) {
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [err, setErr] = useState("");
  const [play, setPlay] = useState<Record<number, number>>({});

  useEffect(() => {
    api.listAlerts(entryId).then(setAlerts).catch((e) => setErr(errorText(e)));
  }, [entryId]);

  const replay = (id: number) => setPlay((p) => ({ ...p, [id]: (p[id] ?? 0) + 1 }));

  const save = async (a: Alert) => {
    setErr("");
    setAlerts((list) => list.map((x) => (x.id === a.id ? a : x)));
    try {
      await api.saveAlert(a);
    } catch (e) {
      setErr(errorText(e));
    }
  };

  const add = async (t: Pick<Alert, "kind" | "threshold" | "notify" | "effect"> = TEMPLATES[0].alert) => {
    setErr("");
    try {
      const a = await api.saveAlert({ id: 0, entryId, windowS: 3600, enabled: true, lastFiredAt: null, ...t });
      setAlerts((l) => [...l, a]);
      replay(a.id);
    } catch (e) {
      setErr(errorText(e));
    }
  };

  const remove = async (id: number) => {
    await api.deleteAlert(id).catch((e) => setErr(errorText(e)));
    setAlerts((l) => l.filter((x) => x.id !== id));
  };

  return (
    <div className="alerts">
      <div className="alert-templates">
        <span className="alert-word">Quick add:</span>
        {TEMPLATES.map((t) => (
          <button type="button" key={t.label} className="pill-btn" onClick={() => add(t.alert)}>
            {t.label}
          </button>
        ))}
      </div>
      {alerts.length === 0 && <p className="muted">No alerts yet. Pick a quick add above, or build your own: a desktop notification, an effect on the widget, or both.</p>}
      {alerts.map((a) => {
        const s = sample(a);
        const notes = checks(a, alerts, view);
        return (
          <div className={"alert-card" + (a.enabled ? "" : " off")} key={a.id}>
            <div className="alert-line">
              <Toggle checked={a.enabled} onChange={(v) => save({ ...a, enabled: v })} ariaLabel="Alert on" />
              <span className="alert-word">When it</span>
              <select
                value={a.kind}
                onChange={(e) => {
                  const kind = e.target.value as AlertKind;
                  save({ ...a, kind, threshold: KINDS.find((k) => k.id === kind)?.def ?? a.threshold });
                }}
              >
                {KINDS.map((k) => (
                  <option key={k.id} value={k.id}>
                    {k.label}
                  </option>
                ))}
              </select>
              <NumberInput
                value={a.threshold}
                min={a.kind === "above" || a.kind === "below" ? undefined : 0}
                onChange={(v) => v !== null && (a.kind === "above" || a.kind === "below" || v > 0) && save({ ...a, threshold: v })}
              />
              {a.kind === "changePct" && (
                <select value={a.windowS} onChange={(e) => save({ ...a, windowS: Number(e.target.value) })}>
                  {WINDOWS.map((w) => (
                    <option key={w.s} value={w.s}>
                      within {w.label}
                    </option>
                  ))}
                </select>
              )}
              <button type="button" className="icon-btn alert-del" aria-label="Delete alert" title="Delete alert" onClick={() => remove(a.id)}>
                ✕
              </button>
            </div>
            <div className="alert-then">
              <div className="alert-then-side">
                <span className="alert-word">then show</span>
                <div className="fx-chips" role="radiogroup" aria-label="Effect on the widget">
                  {EFFECTS.map((x) => (
                    <button
                      type="button"
                      role="radio"
                      aria-checked={a.effect === x.id}
                      key={x.id}
                      className={"fx-chip" + (a.effect === x.id ? " on" : "")}
                      onClick={() => {
                        save({ ...a, effect: x.id });
                        replay(a.id);
                      }}
                    >
                      <span aria-hidden>{x.icon}</span>
                      {x.label}
                    </button>
                  ))}
                </div>
                <label className="alert-check">
                  <input type="checkbox" checked={a.notify} onChange={(e) => save({ ...a, notify: e.target.checked })} />
                  Also send a desktop notification
                </label>
              </div>
              <button type="button" className="fx-preview" onClick={() => replay(a.id)} title="Play the effect" disabled={!a.effect}>
                <FxPreview fx={a.effect} text={s.text} up={s.up} play={play[a.id] ?? 0} />
                <span className="fx-play">{a.effect ? "▶ Play" : "No effect"}</span>
              </button>
            </div>
            {notes.length > 0 && (
              <ul className="alert-notes">
                {notes.map((n) => (
                  <li key={n}>{n}</li>
                ))}
              </ul>
            )}
          </div>
        );
      })}
      <button type="button" className="btn ghost" onClick={() => add()}>
        + Add alert
      </button>
      {err && <span className="field-error">{err}</span>}
    </div>
  );
}
