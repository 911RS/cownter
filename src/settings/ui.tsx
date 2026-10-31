import { useEffect, useState } from "react";

export function Field({ label, hint, children, wide }: { label: string; hint?: React.ReactNode; children: React.ReactNode; wide?: boolean }) {
  return (
    <label className={"field" + (wide ? " wide" : "")}>
      <span className="field-label">{label}</span>
      {children}
      {hint && <span className="field-hint">{hint}</span>}
    </label>
  );
}

export function Toggle({ checked, onChange, label, ariaLabel }: { checked: boolean; onChange: (v: boolean) => void; label?: string; ariaLabel?: string }) {
  return (
    <button type="button" role="switch" aria-checked={checked} aria-label={ariaLabel} className={"toggle" + (checked ? " on" : "")} onClick={() => onChange(!checked)}>
      <span className="knob" />
      {label && <span className="toggle-label">{label}</span>}
    </button>
  );
}

export function Segmented<T extends string>({ value, options, onChange }: { value: T; options: { id: T; label: string }[]; onChange: (v: T) => void }) {
  return (
    <div className="seg" role="radiogroup">
      {options.map((o) => (
        <button type="button" role="radio" aria-checked={o.id === value} key={o.id} className={o.id === value ? "on" : ""} onClick={() => onChange(o.id)}>
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function NumberInput({ value, onChange, placeholder, min, step }: { value: number | null; onChange: (v: number | null) => void; placeholder?: string; min?: number; step?: number }) {
  const [text, setText] = useState(value === null ? "" : String(value));
  useEffect(() => {
    const parsed = text.trim() === "" ? null : Number(text);
    if (parsed !== value) setText(value === null ? "" : String(value));
  }, [value]);
  return (
    <input
      inputMode="decimal"
      value={text}
      placeholder={placeholder}
      min={min}
      step={step}
      onChange={(e) => {
        setText(e.target.value);
        const t = e.target.value.trim().replace(",", ".");
        if (t === "") onChange(null);
        else if (Number.isFinite(Number(t))) onChange(Number(t));
      }}
    />
  );
}

export function Banner({ kind, children }: { kind: "error" | "info" | "ok"; children: React.ReactNode }) {
  return <div className={"banner " + kind}>{children}</div>;
}

const KEY_NAMES: Record<string, string> = { " ": "Space", ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right", Escape: "Escape" };

export function prettyKey(value: string): string {
  return value.replace("CommandOrControl", navigator.platform.includes("Mac") ? "⌘" : "Ctrl").replace(/\+/g, " + ");
}

export function HotkeyInput({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  const [rec, setRec] = useState(false);
  const pretty = prettyKey(value);
  return (
    <div className="hotkey">
      <button
        type="button"
        className={"hotkey-btn" + (rec ? " rec" : "")}
        onClick={() => setRec(true)}
        onBlur={() => setRec(false)}
        onKeyDown={(e) => {
          if (!rec) return;
          e.preventDefault();
          if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;
          if (e.key === "Escape") {
            setRec(false);
            return;
          }
          const mods = [];
          if (e.ctrlKey || e.metaKey) mods.push("CommandOrControl");
          if (e.altKey) mods.push("Alt");
          if (e.shiftKey) mods.push("Shift");
          if (mods.length === 0) return;
          const k = KEY_NAMES[e.key] ?? (e.code.startsWith("Key") ? e.code.slice(3) : e.code.startsWith("Digit") ? e.code.slice(5) : e.key.length === 1 ? e.key.toUpperCase() : e.key);
          onChange([...mods, k].join("+"));
          setRec(false);
        }}
      >
        {rec ? "Press keys…" : value ? pretty : "Not set"}
      </button>
      {value && !rec && (
        <button type="button" className="link" onClick={() => onChange("")}>
          Clear
        </button>
      )}
    </div>
  );
}

export function Group({ title, children }: { title?: string; children: React.ReactNode }) {
  return (
    <section className="group">
      {title && <h3 className="group-title">{title}</h3>}
      <div className="group-body">{children}</div>
    </section>
  );
}

export function Row({ label, hint, children }: { label: React.ReactNode; hint?: React.ReactNode; children?: React.ReactNode }) {
  return (
    <div className="row-item">
      <div className="row-text">
        <span className="row-label">{label}</span>
        {hint && <span className="row-hint">{hint}</span>}
      </div>
      {children !== undefined && <div className="row-control">{children}</div>}
    </div>
  );
}
