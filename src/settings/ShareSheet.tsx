import { useEffect, useRef, useState } from "react";
import { api, errorText } from "../core/api";
import type { EntryView } from "../core/types";
import { canvasBlob, renderSnapshot } from "./share";
import { Banner } from "./ui";

export function ShareSheet({ view, onClose }: { view: EntryView; onClose: () => void }) {
  const canvas = useRef<HTMLCanvasElement | null>(null);
  const [url, setUrl] = useState("");
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<{ kind: "ok" | "error"; text: string } | null>(null);

  useEffect(() => {
    let alive = true;
    renderSnapshot(view)
      .then((c) => {
        if (!alive) return;
        canvas.current = c;
        setUrl(c.toDataURL("image/png"));
      })
      .catch((e) => alive && setNote({ kind: "error", text: errorText(e) }));
    return () => {
      alive = false;
    };
  }, [view.entry.id]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.preventDefault();
      onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const copy = async () => {
    if (!canvas.current) return;
    setBusy(true);
    try {
      const blob = await canvasBlob(canvas.current);
      if (typeof ClipboardItem === "undefined" || !navigator.clipboard?.write) throw new Error("copying images is not supported here; save it instead");
      await navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]);
      setNote({ kind: "ok", text: "Copied to the clipboard." });
    } catch (e) {
      setNote({ kind: "error", text: errorText(e) });
    } finally {
      setBusy(false);
    }
  };

  const save = async () => {
    if (!url) return;
    setBusy(true);
    try {
      const path = await api.saveSnapshot(view.entry.name, url);
      setNote({ kind: "ok", text: `Saved to ${path}` });
    } catch (e) {
      setNote({ kind: "error", text: errorText(e) });
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="share-scrim" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="share-sheet" role="dialog" aria-label={`Share ${view.entry.name}`}>
        <div className="share-preview">{url ? <img src={url} alt={`Snapshot of ${view.entry.name}`} /> : <span className="muted">Drawing…</span>}</div>
        {note && <Banner kind={note.kind}>{note.text}</Banner>}
        <div className="share-actions">
          <button type="button" className="btn ghost" onClick={onClose}>
            Close
          </button>
          <span className="spacer" />
          <button type="button" className="btn" disabled={!url || busy} onClick={copy}>
            Copy image
          </button>
          <button type="button" className="btn primary" disabled={!url || busy} onClick={save}>
            Save to Pictures
          </button>
        </div>
      </div>
    </div>
  );
}
