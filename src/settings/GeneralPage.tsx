import { useEffect, useState } from "react";
import { api } from "../core/api";
import { SOUNDS, playSound } from "../core/sounds";
import type { DisplayConfig } from "../core/types";
import logo from "../../assets/branding/cownter-symbol.png";
import { Group, HotkeyInput, Row, Toggle } from "./ui";

export function GeneralPage({ cfg, keyring, onChange }: { cfg: DisplayConfig; keyring: boolean; onChange: (c: DisplayConfig) => void }) {
  const [info, setInfo] = useState<{ os: string; version: string } | null>(null);
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
        <Row label="Preview" hint="Choose sounds per number in its settings.">
          <div className="sound-grid">
            {SOUNDS.map((s) => (
              <button type="button" key={s.id} className="pill-btn" onClick={() => playSound(s.id, cfg.muted ? 0 : cfg.volume)}>
                {s.label}
              </button>
            ))}
          </div>
        </Row>
      </Group>

      <Group title="Shortcuts">
        <Row label="Show or hide widgets">
          <HotkeyInput value={cfg.hotkeyToggle} onChange={(hotkeyToggle) => onChange({ ...cfg, hotkeyToggle })} />
        </Row>
        <Row label="Refresh all numbers">
          <HotkeyInput value={cfg.hotkeyRefresh} onChange={(hotkeyRefresh) => onChange({ ...cfg, hotkeyRefresh })} />
        </Row>
      </Group>

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
