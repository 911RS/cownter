import { useEffect, useState } from "react";
import { api, errorText } from "../core/api";
import type { Alert, AlertEffect, AlertKind } from "../core/types";
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

const EFFECTS: { id: AlertEffect; label: string }[] = [
  { id: "", label: "No effect" },
  { id: "float", label: "Float up" },
  { id: "pop", label: "Pop bubble" },
  { id: "burst", label: "Burst" },
];

function FxPreview({ fx, text }: { fx: AlertEffect; text: string }) {
  const [n, setN] = useState(0);
  if (!fx) return null;
  return (
    <button type="button" className="fx-preview" onClick={() => setN((x) => x + 1)} title="Play the effect">
      <span className="fx-stage">
        <span key={n} className={`fx fx-${fx} up` + (n === 0 ? " idle" : "")}>
          {fx === "burst" && n > 0 && (
            <span className="fx-sparks">
              {Array.from({ length: 8 }, (_, i) => (
                <i key={i} style={{ "--a": `${i * 45}deg` } as React.CSSProperties} />
              ))}
            </span>
          )}
          <b>{text}</b>
        </span>
      </span>
      ▶
    </button>
  );
}

export function AlertsEditor({ entryId }: { entryId: number }) {
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [err, setErr] = useState("");

  useEffect(() => {
    api.listAlerts(entryId).then(setAlerts).catch((e) => setErr(errorText(e)));
  }, [entryId]);

  const save = async (a: Alert) => {
    setErr("");
    setAlerts((list) => list.map((x) => (x.id === a.id ? a : x)));
    try {
      await api.saveAlert(a);
    } catch (e) {
      setErr(errorText(e));
    }
  };

  const add = async () => {
    setErr("");
    try {
      const a = await api.saveAlert({ id: 0, entryId, kind: "riseBy", threshold: 1, windowS: 3600, enabled: true, lastFiredAt: null, notify: false, effect: "float" });
      setAlerts((l) => [...l, a]);
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
      {alerts.length === 0 && <p className="muted">No alerts yet. Add one to get a notification or an effect on the widget, for example +1 every time the number goes up.</p>}
      {alerts.map((a) => (
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
            <NumberInput value={a.threshold} onChange={(v) => v !== null && save({ ...a, threshold: v })} />
            {a.kind === "changePct" && (
              <select value={a.windowS} onChange={(e) => save({ ...a, windowS: Number(e.target.value) })}>
                {WINDOWS.map((w) => (
                  <option key={w.s} value={w.s}>
                    within {w.label}
                  </option>
                ))}
              </select>
            )}
            <button type="button" className="icon-btn alert-del" aria-label="Delete alert" onClick={() => remove(a.id)}>
              ✕
            </button>
          </div>
          <div className="alert-line">
            <span className="alert-word">then</span>
            <label className="alert-check">
              <input type="checkbox" checked={a.notify} onChange={(e) => save({ ...a, notify: e.target.checked })} />
              Desktop notification
            </label>
            <select value={a.effect} onChange={(e) => save({ ...a, effect: e.target.value as AlertEffect })} aria-label="Effect on the widget">
              {EFFECTS.map((x) => (
                <option key={x.id} value={x.id}>
                  {x.id ? `Effect: ${x.label}` : x.label}
                </option>
              ))}
            </select>
            <FxPreview fx={a.effect} text={a.kind === "dropBy" ? `-${a.threshold}` : `+${a.kind === "riseBy" ? a.threshold : 1}`} />
          </div>
        </div>
      ))}
      <button type="button" className="btn ghost" onClick={add}>
        + Add alert
      </button>
      {err && <span className="field-error">{err}</span>}
    </div>
  );
}
