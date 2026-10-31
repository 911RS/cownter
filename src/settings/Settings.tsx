import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect, useRef, useState } from "react";
import { api, errorText, onSettingsSynced } from "../core/api";
import { useDisplay, useEntriesState } from "../core/hooks";
import type { DisplayConfig, Entry, Group, SyncNotice } from "../core/types";
import symbol from "../../assets/branding/cownter-symbol.png";
import { DisplayPage } from "./DisplayPage";
import { EntryEditor } from "./EntryEditor";
import { GeneralPage } from "./GeneralPage";
import { NumbersPage } from "./NumbersPage";
import { Banner } from "./ui";

type Page = "numbers" | "display" | "general";

export function Settings() {
  const [viewsMap, loaded] = useEntriesState();
  const views = [...viewsMap.values()];
  const [cfg, setCfg] = useDisplay();
  const [groups, setGroups] = useState<Group[]>([]);
  const [keyring, setKeyring] = useState(true);
  const [page, setPage] = useState<Page>("numbers");
  const [editing, setEditing] = useState<Entry | null>(null);
  const [saveErr, setSaveErr] = useState("");
  const [synced, setSynced] = useState<SyncNotice | null>(null);
  const pending = useRef<ReturnType<typeof setTimeout>>(undefined);

  const loadGroups = useCallback(() => {
    api.listGroups().then(setGroups).catch(() => {});
  }, []);

  useEffect(() => {
    loadGroups();
    api.keyringAvailable().then(setKeyring).catch(() => setKeyring(false));
    api.syncInfo().then((i) => setSynced(i.notice)).catch(() => {});
    const un = onSettingsSynced(() => {
      loadGroups();
      api.syncInfo().then((i) => setSynced(i.notice)).catch(() => {});
    });
    return () => {
      un.then((f) => f());
    };
  }, [loadGroups]);

  useEffect(() => {
    if (!editing) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && !e.defaultPrevented && setEditing(null);
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [editing]);

  const updateDisplay = (next: DisplayConfig) => {
    setCfg(next);
    clearTimeout(pending.current);
    pending.current = setTimeout(async () => {
      try {
        await api.saveDisplay(next);
        setSaveErr("");
      } catch (e) {
        setSaveErr(errorText(e));
      }
    }, 250);
  };

  const onSaved = (e: Entry) => {
    setEditing(null);
    if (cfg && !cfg.onboarded) {
      const next = { ...cfg, onboarded: true, tray: { ...cfg.tray, entryId: cfg.tray.entryId ?? e.id } };
      updateDisplay(next);
    }
  };

  return (
    <div className="app">
      <nav className="side">
        <div className="brand" data-tauri-drag-region>
          <img src={symbol} alt="Cownter" />
        </div>
        {(
          [
            ["numbers", "Numbers", views.length],
            ["display", "On screen", null],
            ["general", "General", null],
          ] as const
        ).map(([id, label, count]) => (
          <button type="button" key={id} className={"nav" + (page === id ? " on" : "")} onClick={() => setPage(id)} title={label} aria-label={label}>
            <NavIcon name={id} />
            {count !== null && count > 0 && <span className="count">{count}</span>}
          </button>
        ))}
        <span className="spacer" data-tauri-drag-region />
        {cfg && (
          <button
            type="button"
            className={"nav" + (cfg.hidden ? " dim" : "")}
            title={cfg.hidden ? "Show widgets" : "Hide widgets"}
            aria-label={cfg.hidden ? "Show widgets" : "Hide widgets"}
            onClick={() => updateDisplay({ ...cfg, hidden: !cfg.hidden })}
          >
            <NavIcon name={cfg.hidden ? "hidden" : "visible"} />
          </button>
        )}
      </nav>
      <div className="titlebar" data-tauri-drag-region onDoubleClick={(e) => e.target === e.currentTarget && getCurrentWindow().toggleMaximize()}>
        <button type="button" className="win-btn" aria-label="Minimize" onClick={() => getCurrentWindow().minimize()}>
          <svg viewBox="0 0 10 10" width="10" height="10"><path d="M1 5h8" /></svg>
        </button>
        <button type="button" className="win-btn close" aria-label="Close" onClick={() => getCurrentWindow().close()}>
          <svg viewBox="0 0 10 10" width="10" height="10"><path d="M1.5 1.5l7 7M8.5 1.5l-7 7" /></svg>
        </button>
      </div>
      <main className="main">
        {saveErr && <Banner kind="error">{saveErr}</Banner>}
        {synced && (
          <div className="sync-notice">
            <Banner kind="info">Settings updated from {synced.from}</Banner>
            <button
              type="button"
              className="btn ghost"
              onClick={() => {
                setSynced(null);
                api.syncDismissNotice().catch(() => {});
              }}
            >
              OK
            </button>
          </div>
        )}
        {page === "numbers" && (
          <NumbersPage views={views} groups={groups} onEdit={setEditing} onGroupsChanged={loadGroups} onboarded={!cfg || !loaded || cfg.onboarded || views.length > 0} />
        )}
        {page === "display" && cfg && <DisplayPage cfg={cfg} views={views} groups={groups} onChange={updateDisplay} />}
        {page === "general" && cfg && <GeneralPage cfg={cfg} views={views} keyring={keyring} onChange={updateDisplay} />}
      </main>
      {editing && (
        <div className="scrim" onMouseDown={(e) => e.target === e.currentTarget && setEditing(null)}>
          <EntryEditor key={editing.id || "new"} initial={editing} soundPack={cfg?.soundPack} views={views} groups={groups} keyring={keyring} onClose={() => setEditing(null)} onSaved={onSaved} />
        </div>
      )}
    </div>
  );
}

const NAV_PATHS: Record<string, string> = {
  numbers: "M4 20V10M10 20V4M16 20v-7M22 20H2",
  display: "M3 5h18v11H3zM8 20h8M12 16v4",
  general: "M4 6h10M18 6h2M4 12h4M12 12h8M4 18h12M20 18h0M16 4v4M10 10v4M18 16v4",
  visible: "M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12zM12 15a3 3 0 100-6 3 3 0 000 6z",
  hidden: "M3 3l18 18M10.6 5.1A10 10 0 0112 5c6.4 0 10 7 10 7a17 17 0 01-3.2 4.2M6.6 6.6C3.7 8.4 2 12 2 12s3.6 7 10 7a9.7 9.7 0 005.4-1.6M9.9 9.9a3 3 0 004.2 4.2",
};

function NavIcon({ name }: { name: string }) {
  return (
    <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth={1.9} strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      <path d={NAV_PATHS[name]} />
    </svg>
  );
}
