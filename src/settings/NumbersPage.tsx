import { useState } from "react";
import { api, errorText } from "../core/api";
import { formatDelta, formatValue, goalLeft, intervalLabel, timeAgo } from "../core/format";
import { useNow } from "../core/hooks";
import type { Entry, EntryView, Group } from "../core/types";
import { EntryIcon } from "../components/EntryIcon";
import { Spark } from "../components/Spark";
import { blankEntry } from "./EntryEditor";
import { ShareSheet } from "./ShareSheet";
import { Banner } from "./ui";

const STATUS_LABEL: Record<string, string> = { live: "Live", polling: "Checking", stale: "Out of date", error: "Error", noLink: "Key rejected" };

function starter(name: string, patch: Partial<Entry>): Entry {
  return { ...blankEntry(), name, ...patch };
}

export function NumbersPage({
  views,
  groups,
  onEdit,
  onGroupsChanged,
  onboarded,
}: {
  views: EntryView[];
  groups: Group[];
  onEdit: (e: Entry) => void;
  onGroupsChanged: () => void;
  onboarded: boolean;
}) {
  const now = useNow(15000);
  const [filter, setFilter] = useState<number | "all">("all");
  const [newGroup, setNewGroup] = useState<string | null>(null);
  const [err, setErr] = useState("");
  const [note, setNote] = useState<{ kind: "ok" | "error"; text: string } | null>(null);
  const [sharing, setSharing] = useState<number | null>(null);
  const shared = sharing === null ? undefined : views.find((v) => v.entry.id === sharing);
  const sorted = [...views].sort((a, b) => a.entry.sort - b.entry.sort || a.entry.id - b.entry.id);
  const list = filter === "all" ? sorted : sorted.filter((v) => v.entry.groupId === filter);

  const move = async (id: number, dir: -1 | 1) => {
    const ids = sorted.map((v) => v.entry.id);
    const i = ids.indexOf(id);
    const j = i + dir;
    if (j < 0 || j >= ids.length) return;
    [ids[i], ids[j]] = [ids[j], ids[i]];
    await api.reorderEntries(ids).catch((e) => setErr(errorText(e)));
  };

  const addGroup = async () => {
    if (!newGroup?.trim()) return setNewGroup(null);
    try {
      await api.saveGroup({ id: 0, name: newGroup.trim(), sort: groups.length });
      setNewGroup(null);
      onGroupsChanged();
    } catch (e) {
      setErr(errorText(e));
    }
  };

  const exportCsv = async (entryId: number | null) => {
    try {
      const path = await api.exportCsv(entryId);
      setNote({ kind: "ok", text: `Saved to ${path}` });
    } catch (e) {
      setNote({ kind: "error", text: errorText(e) });
    }
  };

  const removeGroup = async (id: number) => {
    await api.deleteGroup(id).catch((e) => setErr(errorText(e)));
    if (filter === id) setFilter("all");
    onGroupsChanged();
  };

  return (
    <div className="page">
      <header className="page-head" data-tauri-drag-region>
        <h1>Numbers</h1>
        <div className="row">
          <button type="button" className="icon-btn" title="Refresh all" aria-label="Refresh all" onClick={() => api.refreshAll()}>
            <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
              <path d="M20 11a8 8 0 10-2.3 5.7M20 4v7h-7" />
            </svg>
          </button>
          {views.length > 0 && (
            <button type="button" className="btn ghost" title="Save the history of every number as a CSV file" onClick={() => exportCsv(null)}>
              Export CSV
            </button>
          )}
          <button type="button" className="btn primary" onClick={() => onEdit(blankEntry())}>
            + Add
          </button>
        </div>
      </header>

      {!onboarded && views.length === 0 && (
        <div className="welcome">
          <h2>Welcome to Cownter 🐄</h2>
          <p>Pick a starting point. You can change everything later.</p>
          <div className="starters">
            <button type="button" onClick={() => onEdit(starter("GitHub stars", { slug: "github_stars", icon: { kind: "builtin", value: "star", bg: "#24292f" }, source: { type: "github", target: "tauri-apps/tauri", metric: "stars", token: null } }))}>
              <EntryIcon icon={{ kind: "builtin", value: "star", bg: "#24292f" }} size={30} />
              <b>GitHub stars</b>
              <small>No key needed. Try it in 10 seconds.</small>
            </button>
            <button type="button" onClick={() => onEdit(starter("YouTube subscribers", { slug: "youtube_subscribers", icon: { kind: "builtin", value: "play", bg: "#ff2d2d" }, source: { type: "youtube", channelId: "", metric: "subscribers", apiKey: `e${Math.random().toString(36).slice(2, 8)}-apiKey` } }))}>
              <EntryIcon icon={{ kind: "builtin", value: "play", bg: "#ff2d2d" }} size={30} />
              <b>YouTube subscribers</b>
              <small>Needs a free API key.</small>
            </button>
            <button type="button" onClick={() => onEdit(blankEntry())}>
              <EntryIcon icon={{ kind: "builtin", value: "globe", bg: "#22262d" }} size={30} />
              <b>Any URL</b>
              <small>Paste an endpoint, click the number.</small>
            </button>
          </div>
        </div>
      )}

      <div className="tabs">
        <button type="button" className={filter === "all" ? "on" : ""} onClick={() => setFilter("all")}>
          All <span className="count">{views.length}</span>
        </button>
        {groups.map((g) => (
          <span key={g.id} className={"tab-group" + (filter === g.id ? " on" : "")}>
            <button type="button" onClick={() => setFilter(g.id)}>
              {g.name} <span className="count">{views.filter((v) => v.entry.groupId === g.id).length}</span>
            </button>
            <button type="button" className="tab-x" aria-label={`Delete group ${g.name}`} onClick={() => removeGroup(g.id)}>
              ✕
            </button>
          </span>
        ))}
        {newGroup === null ? (
          <button type="button" className="tab-add" onClick={() => setNewGroup("")}>
            + Group
          </button>
        ) : (
          <input
            autoFocus
            className="tab-input"
            value={newGroup}
            placeholder="Group name"
            onChange={(e) => setNewGroup(e.target.value)}
            onBlur={addGroup}
            onKeyDown={(e) => {
              if (e.key === "Enter") addGroup();
              if (e.key === "Escape") setNewGroup(null);
            }}
          />
        )}
      </div>

      {err && <Banner kind="error">{err}</Banner>}
      {note && <Banner kind={note.kind}>{note.text}</Banner>}

      <div className="entries">
        {list.length === 0 && views.length > 0 && <p className="muted pad">No numbers in this group yet.</p>}
        {list.map((v) => (
          <div className="entry-row" key={v.entry.id} onClick={() => onEdit(v.entry)} role="button" tabIndex={0} onKeyDown={(e) => e.key === "Enter" && onEdit(v.entry)}>
            <div className="order" onClick={(e) => e.stopPropagation()}>
              <button type="button" aria-label="Move up" onClick={() => move(v.entry.id, -1)}>
                ▲
              </button>
              <button type="button" aria-label="Move down" onClick={() => move(v.entry.id, 1)}>
                ▼
              </button>
            </div>
            <EntryIcon icon={v.entry.icon} size={28} />
            <div className="entry-name">
              <b>{v.entry.name}</b>
              <span className={"entry-sub " + v.status} title={v.error ?? undefined}>
                <i className="sdot" />
                {v.status === "live" || v.status === "polling"
                  ? `${v.entry.source.type === "formula" ? "Formula" : v.entry.source.type === "push" ? "Push" : intervalLabel(v.entry.intervalS)} · ${timeAgo(v.updatedAt, now)}`
                  : `${STATUS_LABEL[v.status]}${v.error ? ": " + v.error : ""}`}
              </span>
            </div>
            <span className="entry-spark">
              <Spark points={v.spark} width={90} height={24} color={(v.deltaToday ?? 0) < 0 ? "#ff6b6b" : "#5b8cff"} />
            </span>
            <div className="entry-value">
              <b>{formatValue(v.value, v.entry.format)}</b>
              <span className={(v.deltaToday ?? 0) >= 0 ? "pos" : "neg"}>
                {formatDelta(v.deltaToday, v.entry.format) || " "}
                {(v.streak ?? 0) >= 2 && (
                  <em className="streak-mark" title={`Up ${v.streak} days in a row`}>
                    🔥 {v.streak}
                  </em>
                )}
              </span>
            </div>
            <div className="goal" title={v.goalProgress !== null ? [`Goal ${formatValue(v.entry.goalValue, v.entry.format)}`, goalLeft(v)].filter(Boolean).join(" · ") : undefined}>
              {v.goalProgress !== null && (
                <>
                  <div className="goal-bar">
                    <i style={{ width: `${Math.round(v.goalProgress * 100)}%` }} />
                  </div>
                  <small>{Math.round(v.goalProgress * 100)}%</small>
                </>
              )}
            </div>
            <button
              type="button"
              className="icon-btn share-btn"
              title="Share a snapshot"
              aria-label={`Share ${v.entry.name}`}
              onClick={(e) => {
                e.stopPropagation();
                setSharing(v.entry.id);
              }}
              onKeyDown={(e) => e.stopPropagation()}
            >
              <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
                <path d="M12 15V3M7 8l5-5 5 5M5 13v6a2 2 0 002 2h10a2 2 0 002-2v-6" />
              </svg>
            </button>
            <button
              type="button"
              className="icon-btn share-btn"
              title="Export CSV"
              aria-label={`Export ${v.entry.name} as CSV`}
              onClick={(e) => {
                e.stopPropagation();
                exportCsv(v.entry.id);
              }}
              onKeyDown={(e) => e.stopPropagation()}
            >
              <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
                <path d="M12 4v11M7 10.5l5 5 5-5M4 20h16" />
              </svg>
            </button>
          </div>
        ))}
      </div>
      {shared && <ShareSheet view={shared} onClose={() => setSharing(null)} />}
    </div>
  );
}
