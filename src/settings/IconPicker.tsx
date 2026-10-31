import { useState } from "react";
import { api, errorText } from "../core/api";
import type { Icon } from "../core/types";
import { EntryIcon } from "../components/EntryIcon";
import { BUILTIN_ICONS, EMOJIS, ICON_COLORS } from "../components/icons";
import { Segmented } from "./ui";

const MAX_UPLOAD = 256 * 1024;

export function IconPicker({ icon, onChange, siteUrl }: { icon: Icon; onChange: (i: Icon) => void; siteUrl?: string }) {
  const [tab, setTab] = useState<"emoji" | "builtin" | "image">(icon.kind === "builtin" ? "builtin" : icon.kind === "emoji" ? "emoji" : "image");
  const [err, setErr] = useState("");
  const [busy, setBusy] = useState(false);
  const [favUrl, setFavUrl] = useState(siteUrl ?? "");

  const upload = (f: File | undefined) => {
    setErr("");
    if (!f) return;
    if (f.size > MAX_UPLOAD) return setErr("Pick an image under 256 KB.");
    if (!/^image\/(png|jpeg|svg\+xml|webp|gif|x-icon|vnd\.microsoft\.icon)$/.test(f.type)) return setErr("Use a PNG, JPG, SVG, WebP, GIF or ICO file.");
    const r = new FileReader();
    r.onload = () => onChange({ ...icon, kind: "image", value: String(r.result) });
    r.readAsDataURL(f);
  };

  const favicon = async () => {
    setErr("");
    setBusy(true);
    try {
      const url = /^https?:\/\//.test(favUrl.trim()) ? favUrl.trim() : `https://${favUrl.trim()}`;
      onChange({ ...icon, kind: "favicon", value: await api.fetchFavicon(url) });
    } catch (e) {
      setErr(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="icon-picker">
      <div className="icon-preview">
        <EntryIcon icon={icon} size={44} />
        <div className="colors">
          {ICON_COLORS.map((c) => (
            <button type="button" key={c} aria-label={`Background ${c}`} className={"color" + (icon.bg === c ? " on" : "")} style={{ background: c }} onClick={() => onChange({ ...icon, bg: c })} />
          ))}
          <input type="color" aria-label="Custom colour" value={icon.bg.startsWith("#") && icon.bg.length === 7 ? icon.bg : "#2b2f3a"} onChange={(e) => onChange({ ...icon, bg: e.target.value })} />
        </div>
      </div>
      <Segmented
        value={tab}
        onChange={setTab}
        options={[
          { id: "emoji", label: "Emoji" },
          { id: "builtin", label: "Icons" },
          { id: "image", label: "Image / favicon" },
        ]}
      />
      {tab === "emoji" && (
        <div className="icon-grid">
          {EMOJIS.map((e) => (
            <button type="button" key={e} className={"icon-cell" + (icon.kind === "emoji" && icon.value === e ? " on" : "")} onClick={() => onChange({ ...icon, kind: "emoji", value: e })}>
              {e}
            </button>
          ))}
          <input
            className="emoji-input"
            placeholder="Any emoji"
            maxLength={8}
            onChange={(e) => e.target.value.trim() && onChange({ ...icon, kind: "emoji", value: e.target.value.trim() })}
          />
        </div>
      )}
      {tab === "builtin" && (
        <div className="icon-grid">
          {Object.keys(BUILTIN_ICONS).map((k) => (
            <button type="button" key={k} title={k} className={"icon-cell" + (icon.kind === "builtin" && icon.value === k ? " on" : "")} onClick={() => onChange({ ...icon, kind: "builtin", value: k })}>
              <EntryIcon icon={{ kind: "builtin", value: k, bg: "#22262d" }} size={26} />
            </button>
          ))}
        </div>
      )}
      {tab === "image" && (
        <div className="icon-image">
          <label className="btn">
            Upload image
            <input type="file" accept="image/png,image/jpeg,image/svg+xml,image/webp,image/gif,image/x-icon" hidden onChange={(e) => upload(e.target.files?.[0])} />
          </label>
          <div className="row">
            <input value={favUrl} placeholder="example.com" onChange={(e) => setFavUrl(e.target.value)} />
            <button type="button" className="btn" disabled={busy || !favUrl.trim()} onClick={favicon}>
              {busy ? "Fetching…" : "Use site icon"}
            </button>
          </div>
        </div>
      )}
      {err && <span className="field-error">{err}</span>}
    </div>
  );
}
