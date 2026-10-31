import { useMemo, useState } from "react";
import { isNumericLeaf, joinPath } from "../core/jsonpath";
import type { Pick, TestResult } from "../core/types";

interface NodeProps {
  name: string | number | null;
  value: unknown;
  path: (string | number)[];
  selected: string;
  onPick: (p: string) => void;
  depth: number;
  text?: boolean;
}

const MAX_CHILDREN = 200;

function Node({ name, value, path, selected, onPick, depth, text }: NodeProps) {
  const [open, setOpen] = useState(depth < 3);
  const label = name === null ? null : <span className="jt-key">{typeof name === "number" ? `[${name}]` : name}</span>;
  if (value !== null && typeof value === "object") {
    const entries: [string | number, unknown][] = Array.isArray(value) ? value.map((v, i) => [i, v]) : Object.entries(value as object);
    const brace = Array.isArray(value) ? ["[", "]"] : ["{", "}"];
    return (
      <div className="jt-node">
        <button type="button" className="jt-toggle" onClick={() => setOpen(!open)}>
          {open ? "▾" : "▸"} {label} <span className="jt-dim">{brace[0]}{open ? "" : ` ${entries.length} ${brace[1]}`}</span>
        </button>
        {open && (
          <div className="jt-children">
            {entries.slice(0, MAX_CHILDREN).map(([k, v]) => (
              <Node key={String(k)} name={k} value={v} path={[...path, k]} selected={selected} onPick={onPick} depth={depth + 1} text={text} />
            ))}
            {entries.length > MAX_CHILDREN && <div className="jt-dim">… {entries.length - MAX_CHILDREN} more</div>}
            <span className="jt-dim">{brace[1]}</span>
          </div>
        )}
      </div>
    );
  }
  const p = joinPath(path);
  const numeric = text ? value !== null && value !== "" : isNumericLeaf(value);
  return (
    <div className="jt-leaf">
      {label}
      {label && <span className="jt-dim">: </span>}
      {numeric ? (
        <button type="button" className={"jt-num" + (p === selected ? " on" : "")} onClick={() => onPick(p)} title={`Track ${p || "this value"}`}>
          {JSON.stringify(value)}
        </button>
      ) : (
        <span className="jt-val">{JSON.stringify(value)}</span>
      )}
    </div>
  );
}

export function SmartPicker({ result, pick, onPick, text }: { result: TestResult; pick: Pick; onPick: (p: Pick) => void; text?: boolean }) {
  const json = useMemo(() => {
    try {
      return { ok: true as const, v: JSON.parse(result.body) as unknown };
    } catch {
      return { ok: false as const };
    }
  }, [result.body]);
  const selected = pick.kind === "jsonPath" ? pick.value : "\u0000";

  return (
    <div className="picker">
      <div className="picker-head">
        <span className={"http " + (result.status < 300 ? "ok" : "bad")}>HTTP {result.status}</span>
        <span className="jt-dim">{result.elapsedMs} ms{result.contentType ? ` · ${result.contentType}` : ""}</span>
        {text && result.text ? (
          <span className="picker-value">
            Text: <b>{result.text}</b>
          </span>
        ) : !text && result.value !== null ? (
          <span className="picker-value">
            Value: <b>{result.value.toLocaleString()}</b>
          </span>
        ) : (
          result.error && <span className="picker-err">{result.error}</span>
        )}
      </div>
      {json.ok ? (
        <>
          <p className="picker-tip">{text ? "Click the value you want to show." : "Click the number you want to track."}</p>
          <div className="jt">
            <Node name={null} value={json.v} path={[]} selected={selected} onPick={(p) => onPick({ kind: "jsonPath", value: p })} depth={0} text={text} />
          </div>
        </>
      ) : (
        <>
          <p className="picker-tip">{text ? "This response is not JSON. Use the first line, or a pattern." : "This response is not JSON. Use the first number in it, or a pattern."}</p>
          <pre className="raw">{result.body.slice(0, 4000) || "(empty response)"}</pre>
        </>
      )}
      {result.truncated && <p className="jt-dim">Preview cut at 256 KB.</p>}
    </div>
  );
}
