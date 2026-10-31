import { useEffect, useRef, useState } from "react";
import { api } from "../core/api";
import { formatValue } from "../core/format";
import { useDisplay, useEntries } from "../core/hooks";
import { playSound, resolveSound } from "../core/sounds";
import type { EntryView } from "../core/types";
import { EntryIcon } from "../components/EntryIcon";
import { Skin } from "../skins/Skin";

const SHOW_MS = 4800;

interface Party {
  key: number;
  view: EntryView;
  value: number;
  text: string | null;
  kind: "goal" | "record";
}

function Confetti() {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const c = ref.current;
    if (!c) return;
    const ctx = c.getContext("2d");
    if (!ctx) return;
    c.width = window.innerWidth;
    c.height = window.innerHeight;
    const colors = ["#ffb340", "#5dff8a", "#5ad1ff", "#ff5a7a", "#b277ff", "#fff3b0"];
    const parts = Array.from({ length: 220 }, () => ({
      x: c.width / 2 + (Math.random() - 0.5) * 200,
      y: c.height * 0.42,
      vx: (Math.random() - 0.5) * 22,
      vy: -Math.random() * 20 - 6,
      r: Math.random() * Math.PI,
      vr: (Math.random() - 0.5) * 0.3,
      w: 6 + Math.random() * 6,
      h: 8 + Math.random() * 10,
      color: colors[Math.floor(Math.random() * colors.length)],
    }));
    let raf = 0;
    const start = performance.now();
    const tick = (t: number) => {
      const age = (t - start) / 1000;
      ctx.clearRect(0, 0, c.width, c.height);
      for (const p of parts) {
        p.vy += 0.45;
        p.vx *= 0.99;
        p.x += p.vx;
        p.y += p.vy;
        p.r += p.vr;
        ctx.save();
        ctx.globalAlpha = Math.max(0, 1 - Math.max(0, age - 3) / 1.5);
        ctx.translate(p.x, p.y);
        ctx.rotate(p.r);
        ctx.fillStyle = p.color;
        ctx.fillRect(-p.w / 2, -p.h / 2, p.w, p.h * Math.abs(Math.cos(p.r * 2)));
        ctx.restore();
      }
      if (age < 4.6) raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, []);
  return <canvas ref={ref} className="confetti" />;
}

/** Hidden hub window: plays sounds for every window and shows the full-screen celebration. */
export function Overlay() {
  const [cfg] = useDisplay();
  const cfgRef = useRef(cfg);
  cfgRef.current = cfg;
  const [party, setParty] = useState<Party | null>(null);
  const seq = useRef(0);

  useEntries((e, views) => {
    const c = cfgRef.current;
    if (!c) return;
    const vol = c.muted ? 0 : c.volume;
    if (e.type === "valueChanged" || e.type === "textChanged") {
      const s = views.get(e.entryId)?.entry.sounds;
      playSound(resolveSound("change", s?.onChange, s?.pack, c.soundPack), vol);
      return;
    }
    if (e.type !== "goalReached" && e.type !== "allTimeHigh") return;
    const view = views.get(e.entryId);
    if (!view) return;
    if (e.type === "goalReached" || view.entry.sounds.onGoal) {
      const s = view.entry.sounds;
      playSound(resolveSound("goal", s.onGoal ?? (view.entry.celebrate ? "sparkle" : null), s.pack, c.soundPack), vol);
    }
    if (c.celebration && view.entry.celebrate) {
      setParty({ key: ++seq.current, view, value: e.value, text: e.type === "goalReached" ? (e.text ?? null) : null, kind: e.type === "goalReached" ? "goal" : "record" });
    }
  });

  useEffect(() => {
    if (!party) return;
    api.widgetResize(1, 1, true).catch(() => {});
    const t = setTimeout(() => {
      setParty(null);
      api.widgetDone().catch(() => {});
    }, SHOW_MS);
    return () => clearTimeout(t);
  }, [party?.key]);

  if (!party || !cfg) return null;
  const v = party.view;
  return (
    <div className="overlay" key={party.key}>
      <Confetti />
      <div className="party-card">
        <small>{party.kind === "goal" ? "GOAL REACHED" : "NEW RECORD"}</small>
        <Skin skin={cfg.island.skin} tint={cfg.island.tint} opts={cfg.island.skinOpts} text={party.text ?? formatValue(party.value, v.entry.format)} height={56} />
        <span className="party-name">
          <EntryIcon icon={v.entry.icon} size={22} /> {v.entry.name}
        </span>
      </div>
    </div>
  );
}
