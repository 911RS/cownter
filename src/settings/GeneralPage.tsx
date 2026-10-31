import { useEffect, useState } from "react";
import { api, errorText, onPushChanged } from "../core/api";
import { PACKS, ROLE_DEFAULT, SOUNDS, playSound, resolveSound } from "../core/sounds";
import type { DisplayConfig, EntryView, PushInfo } from "../core/types";
import logo from "../../assets/branding/cownter-symbol.png";
import { curlExample, maskToken, parsePort } from "./push";
import { Banner, Group, HotkeyInput, Row, Toggle } from "./ui";

const WEEKDAYS = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

function copy(text: string, done: () => void) {
  navigator.clipboard.writeText(text).then(done).catch(() => {});
}

function PushGroup({ cfg, views, onChange }: { cfg: DisplayConfig; views: EntryView[]; onChange: (c: DisplayConfig) => void }) {
  const [info, setInfo] = useState<PushInfo | null>(null);
  const [port, setPort] = useState(String(cfg.pushPort));
  const [shown, setShown] = useState(false);
  const [armed, setArmed] = useState(false);
  const [copied, setCopied] = useState("");
  const [err, setErr] = useState("");

  useEffect(() => {
    api.pushInfo().then(setInfo).catch((e) => setErr(errorText(e)));
    const un = onPushChanged(setInfo);
    return () => {
      un.then((f) => f());
    };
  }, []);
  useEffect(() => setPort(String(cfg.pushPort)), [cfg.pushPort]);
  useEffect(() => {
    if (!copied) return;
    const t = setTimeout(() => setCopied(""), 1500);
    return () => clearTimeout(t);
  }, [copied]);

  const commitPort = () => {
    const p = parsePort(port);
    if (p === null) {
      setErr("Use a port from 1024 to 65535.");
      setPort(String(cfg.pushPort));
      return;
    }
    setErr("");
    if (p !== cfg.pushPort) onChange({ ...cfg, pushPort: p });
  };
  const regenerate = () => {
    if (!armed) return setArmed(true);
    setArmed(false);
    api.regeneratePushToken().then(setInfo).catch((e) => setErr(errorText(e)));
  };

  const name = views.find((v) => v.entry.source.type === "push")?.entry.name ?? "Orders";
  const url = info?.url ?? `http://127.0.0.1:${cfg.pushPort}`;
  const token = info?.token ?? "";
  return (
    <Group title="Push and command line">
      <Row label="Address" hint="Push numbers get their value from requests to this address. Only programs on this computer can reach it.">
        <span className="mono">{url}</span>
        <span className={"dot-status " + (info?.running ? "good" : "bad")}>{info?.running ? "Listening" : "Off"}</span>
      </Row>
      {info?.error && (
        <div className="push-error">
          <Banner kind="error">{info.error}</Banner>
          <button type="button" className="btn" onClick={() => api.restartPush().then(setInfo).catch((e) => setErr(errorText(e)))}>
            Try again
          </button>
        </div>
      )}
      <Row label="Port">
        <input className="mono port-input" inputMode="numeric" value={port} onChange={(e) => setPort(e.target.value.replace(/[^0-9]/g, "").slice(0, 5))} onBlur={commitPort} onKeyDown={(e) => e.key === "Enter" && commitPort()} />
      </Row>
      <Row label="Token" hint="Send it as Authorization: Bearer <token>. Kept in your keyring when one is available.">
        <span className="mono token">{shown ? token : maskToken(token)}</span>
        <button type="button" className="btn ghost" onClick={() => setShown(!shown)} disabled={!token}>
          {shown ? "Hide" : "Show"}
        </button>
        <button type="button" className="btn" onClick={() => copy(token, () => setCopied("token"))} disabled={!token}>
          {copied === "token" ? "Copied" : "Copy"}
        </button>
        <button type="button" className={"btn danger" + (armed ? " armed" : "")} onClick={regenerate} onBlur={() => setArmed(false)}>
          {armed ? "Old token stops working" : "Regenerate"}
        </button>
      </Row>
      <div className="push-example-row">
        <div className="row-text">
          <span className="row-label">Try it</span>
          <span className="row-hint">
            Or use the command line: <code className="mono">cownter set "{name}" 42</code>, <code className="mono">cownter add "{name}" 1</code>, <code className="mono">cownter get "{name}"</code>, <code className="mono">cownter list</code>.
          </span>
        </div>
        <pre className="mono push-curl">{curlExample(url, shown ? token : maskToken(token), name)}</pre>
        <button type="button" className="btn" onClick={() => copy(curlExample(url, token, name), () => setCopied("curl"))} disabled={!token}>
          {copied === "curl" ? "Copied" : "Copy command"}
        </button>
      </div>
      {err && (
        <div className="push-note">
          <Banner kind="error">{err}</Banner>
        </div>
      )}
    </Group>
  );
}

export function GeneralPage({ cfg, views, keyring, onChange }: { cfg: DisplayConfig; views: EntryView[]; keyring: boolean; onChange: (c: DisplayConfig) => void }) {
  const [info, setInfo] = useState<{ os: string; version: string } | null>(null);
  const [recapErr, setRecapErr] = useState("");
  useEffect(() => {
    api.platformInfo().then(setInfo).catch(() => {});
  }, []);
  return (
    <div className="page">
      <header className="page-head" data-tauri-drag-region>
        <h1>General</h1>
      </header>

      <Group title="Sound">
        <Row label="Volume" hint={cfg.muted ? "Muted" : `${Math.round(cfg.volume * 100)}%`}>
          <input type="range" min={0} max={100} value={Math.round(cfg.volume * 100)} disabled={cfg.muted} onChange={(e) => onChange({ ...cfg, volume: Number(e.target.value) / 100 })} />
        </Row>
        <Row label="Mute all sounds">
          <Toggle checked={cfg.muted} onChange={(muted) => onChange({ ...cfg, muted })} />
        </Row>
        <Row label="Sound pack" hint="Soft plays the sound each number picks. Other packs give every number the pack's own sounds; a number can pick its own pack.">
          <select value={cfg.soundPack} onChange={(e) => onChange({ ...cfg, soundPack: e.target.value })}>
            {PACKS.map((p) => (
              <option key={p.id} value={p.id}>
                {p.label}
              </option>
            ))}
          </select>
        </Row>
        <Row label="Preview pack">
          <div className="sound-grid">
            {PACKS.map((p) => (
              <span key={p.id} className="pack-preview">
                <span className="pack-name">{p.label}</span>
                {(["change", "goal"] as const).map((role) => (
                  <button type="button" key={role} className="pill-btn" onClick={() => playSound(resolveSound(role, ROLE_DEFAULT[role], p.id), cfg.muted ? 0 : cfg.volume)}>
                    {role === "change" ? "Change" : "Goal"}
                  </button>
                ))}
              </span>
            ))}
          </div>
        </Row>
        <Row label="Soft sounds" hint="Choose sounds per number in its settings.">
          <div className="sound-grid">
            {SOUNDS.map((s) => (
              <button type="button" key={s.id} className="pill-btn" onClick={() => playSound(s.id, cfg.muted ? 0 : cfg.volume)}>
                {s.label}
              </button>
            ))}
          </div>
        </Row>
      </Group>

      <Group title="Weekly recap">
        <Row label="Weekly recap" hint="A card with how each number moved over the last 7 days, shown once a week after 9:00.">
          <Toggle checked={cfg.weeklyRecap} onChange={(weeklyRecap) => onChange({ ...cfg, weeklyRecap })} />
        </Row>
        <Row label="Day">
          <select value={cfg.recapWeekday} disabled={!cfg.weeklyRecap} onChange={(e) => onChange({ ...cfg, recapWeekday: Number(e.target.value) })}>
            {WEEKDAYS.map((d, i) => (
              <option key={d} value={i}>
                {d}
              </option>
            ))}
          </select>
        </Row>
        <Row label="Preview" hint="Shows the card where pop-ups appear.">
          <button type="button" className="btn" onClick={() => api.showRecap().then(() => setRecapErr("")).catch((e) => setRecapErr(errorText(e)))}>
            Show recap now
          </button>
        </Row>
        {recapErr && <Banner kind="error">{recapErr}</Banner>}
      </Group>

      <Group title="Shortcuts">
        <Row label="Show or hide widgets">
          <HotkeyInput value={cfg.hotkeyToggle} onChange={(hotkeyToggle) => onChange({ ...cfg, hotkeyToggle })} />
        </Row>
        <Row label="Refresh all numbers">
          <HotkeyInput value={cfg.hotkeyRefresh} onChange={(hotkeyRefresh) => onChange({ ...cfg, hotkeyRefresh })} />
        </Row>
      </Group>

      <PushGroup cfg={cfg} views={views} onChange={onChange} />

      <Group title="System">
        <Row label="Start at login">
          <Toggle checked={cfg.launchAtLogin} onChange={(launchAtLogin) => onChange({ ...cfg, launchAtLogin })} />
        </Row>
        <Row
          label="Keys and tokens"
          hint={keyring ? "Stored in your system keyring, never in Cownter's files." : "No system keyring found. Sources that need a key can't be saved."}
        >
          <span className={"dot-status " + (keyring ? "good" : "bad")}>{keyring ? "Secure" : "Unavailable"}</span>
        </Row>
        <Row label="Privacy" hint="No account, no server, no telemetry. Cownter only calls the addresses you add." />
      </Group>

      <Group>
        <div className="about">
          <img src={logo} alt="Cownter" className="about-logo" />
          <div className="row-text">
            <span className="row-label">Cownter {info?.version ?? ""}</span>
            <span className="row-hint">Free and open source · MIT License</span>
          </div>
          <span className="spacer" />
          <button type="button" className="btn ghost" onClick={() => api.quit()}>
            Quit
          </button>
        </div>
      </Group>
    </div>
  );
}
