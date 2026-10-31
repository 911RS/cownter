import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { api } from "../core/api";

interface Size {
  w: number;
  h: number;
}

function useSize(el: HTMLElement | null): Size | null {
  const [s, setS] = useState<Size | null>(null);
  useLayoutEffect(() => {
    if (!el) return setS(null);
    const read = () =>
      setS((p) => {
        const w = el.offsetWidth;
        const h = el.offsetHeight;
        return p && p.w === w && p.h === h ? p : { w, h };
      });
    read();
    const ro = new ResizeObserver(read);
    ro.observe(el);
    return () => ro.disconnect();
  }, [el]);
  return s;
}

function place(total: number, size: number, side: "start" | "mid" | "end") {
  return side === "start" ? 0 : side === "end" ? total - size : (total - size) / 2;
}

interface Props {
  /** Real widget window: reports its hit area to the app. Off in settings previews. */
  live: boolean;
  open: boolean;
  /** Window anchor, e.g. "topCenter"; the panel grows away from it. */
  position: string;
  className: string;
  openClass?: string;
  closed: ReactNode;
  opened: ReactNode | null;
  onEnter?: () => void;
  onLeave?: () => void;
  onClick?: () => void;
  title?: string;
}

/**
 * A panel that morphs between a closed and an open layout inside a window that keeps the open
 * size, so hovering never resizes the native window. Only the panel takes mouse input.
 */
export function Morph({ live, open, position, className, openClass = "open", closed, opened, onEnter, onLeave, onClick, title }: Props) {
  const [a, setA] = useState<HTMLDivElement | null>(null);
  const [b, setB] = useState<HTMLDivElement | null>(null);
  const [stage, setStage] = useState<HTMLDivElement | null>(null);
  const [armed, setArmed] = useState(false);
  const cs = useSize(a);
  const os = useSize(opened ? b : null);
  const isOpen = open && !!os;
  const leave = useRef(onLeave);
  leave.current = onLeave;

  const hs = position.endsWith("Left") ? "start" : position.endsWith("Right") ? "end" : "mid";
  const vs = position.startsWith("bottom") ? "end" : position.startsWith("middle") ? "mid" : "start";
  const W = Math.max(cs?.w ?? 0, os?.w ?? 0);
  const H = Math.max(cs?.h ?? 0, os?.h ?? 0);
  const cur = (isOpen ? os : cs) ?? { w: 0, h: 0 };
  const px = place(W, cur.w, hs);
  const py = place(H, cur.h, vs);
  const layer = (s: Size | null) => ({ left: place(W, s?.w ?? 0, hs) - px, top: place(H, s?.h ?? 0, vs) - py });

  useEffect(() => {
    if (!cs || armed) return;
    const id = requestAnimationFrame(() => requestAnimationFrame(() => setArmed(true)));
    return () => cancelAnimationFrame(id);
  }, [cs, armed]);

  useEffect(() => {
    if (!live || !stage || !cs) return;
    const r = stage.getBoundingClientRect();
    api.widgetHit({ x: r.left + px, y: r.top + py, w: cur.w, h: cur.h }).catch(() => {});
  }, [live, stage, cs, px, py, cur.w, cur.h]);

  useEffect(() => {
    if (!live) return;
    const un = getCurrentWebviewWindow().listen("pointer-out", () => leave.current?.());
    return () => {
      un.then((u) => u());
    };
  }, [live]);

  const origin = `${hs === "start" ? "left" : hs === "end" ? "right" : "center"} ${vs === "start" ? "top" : vs === "end" ? "bottom" : "center"}`;

  return (
    <div className={"m-stage" + (armed ? " armed" : "")} ref={setStage} style={{ width: W, height: H, visibility: cs ? undefined : "hidden" }}>
      <div
        className={className + " m-panel" + (isOpen ? " " + openClass : "")}
        style={{ left: px, top: py, width: cur.w, height: cur.h }}
        onMouseEnter={onEnter}
        onMouseLeave={onLeave}
        onClick={onClick}
        title={title}
      >
        <div className="m-clip">
          <div className={"m-layer m-closed" + (isOpen ? " off" : "")} ref={setA} style={{ ...layer(cs), transformOrigin: origin }} aria-hidden={isOpen}>
            {closed}
          </div>
          {opened && (
            <div className={"m-layer m-open" + (isOpen ? "" : " off")} ref={setB} style={{ ...layer(os), transformOrigin: origin }} aria-hidden={!isOpen}>
              {opened}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
