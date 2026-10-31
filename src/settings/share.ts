import { BUILTIN_ICONS } from "../components/icons";
import { light } from "../components/EntryIcon";
import { formatDelta, formatValue, goalLeft } from "../core/format";
import type { EntryView, Icon } from "../core/types";
import { valueText } from "../forms/common";

const W = 600;
const H = 380;
const PAD = 36;
const SCALE = 2;
const FONT = '"Inter Tight", system-ui, sans-serif';
const EMOJI_FONT = '"Apple Color Emoji", "Segoe UI Emoji", "Noto Color Emoji", sans-serif';

export interface ShareDetails {
  value: string;
  delta: string;
  up: boolean;
  streak: string;
  goal: { progress: number; label: string; left: string } | null;
}

/** The texts drawn on a snapshot card. */
export function shareDetails(v: EntryView, now = Date.now()): ShareDetails {
  const d = v.entry.textMode ? "" : formatDelta(v.deltaToday, v.entry.format);
  const n = v.streak ?? 0;
  const goal =
    v.goalProgress !== null && v.entry.goalValue !== null && !v.entry.textMode
      ? {
          progress: v.goalProgress,
          label: `Goal ${formatValue(v.entry.goalValue, v.entry.format)} · ${Math.round(v.goalProgress * 100)}%`,
          left: v.goalProgress >= 1 ? "Reached" : goalLeft(v, now),
        }
      : null;
  return { value: valueText(v), delta: d ? `${d} today` : "", up: (v.deltaToday ?? 0) >= 0, streak: n >= 2 ? `🔥 ${n}-day streak` : "", goal };
}

/** Sparkline points mapped into the box, oldest on the left. */
export function sparkCoords(points: [number, number][], x: number, y: number, w: number, h: number): [number, number][] {
  if (points.length < 2) return [];
  const xs = points.map((p) => p[0]);
  const ys = points.map((p) => p[1]);
  const [x0, x1] = [Math.min(...xs), Math.max(...xs)];
  const [y0, y1] = [Math.min(...ys), Math.max(...ys)];
  return points.map(([t, v]) => [x1 === x0 ? x + w / 2 : x + ((t - x0) / (x1 - x0)) * w, y1 === y0 ? y + h / 2 : y + h - ((v - y0) / (y1 - y0)) * h]);
}

/** Cuts `text` with an ellipsis so `measure(text)` fits in `max`. */
export function ellipsize(text: string, max: number, measure: (t: string) => number): string {
  if (measure(text) <= max) return text;
  const chars = [...text];
  while (chars.length > 1 && measure(chars.join("").trimEnd() + "…") > max) chars.pop();
  return chars.join("").trimEnd() + "…";
}

function rounded(g: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  const rr = Math.min(r, w / 2, h / 2);
  g.beginPath();
  g.moveTo(x + rr, y);
  g.arcTo(x + w, y, x + w, y + h, rr);
  g.arcTo(x + w, y + h, x, y + h, rr);
  g.arcTo(x, y + h, x, y, rr);
  g.arcTo(x, y, x + w, y, rr);
  g.closePath();
}

function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error("icon could not be loaded"));
    img.src = src;
  });
}

async function drawIcon(g: CanvasRenderingContext2D, icon: Icon, x: number, y: number, size: number) {
  rounded(g, x, y, size, size, size * 0.3);
  g.fillStyle = icon.bg;
  g.fill();
  if (icon.kind === "builtin") {
    g.save();
    g.translate(x + size * 0.17, y + size * 0.17);
    g.scale((size * 0.66) / 24, (size * 0.66) / 24);
    g.strokeStyle = light(icon.bg) ? "#14161b" : "#fff";
    g.lineWidth = 2.2;
    g.lineCap = "round";
    g.lineJoin = "round";
    g.stroke(new Path2D(BUILTIN_ICONS[icon.value] ?? BUILTIN_ICONS.chart));
    g.restore();
  } else if (icon.kind === "image" || icon.kind === "favicon") {
    try {
      const img = await loadImage(icon.value);
      const box = size * 0.72;
      const k = Math.min(box / (img.naturalWidth || box), box / (img.naturalHeight || box));
      const [w, h] = [(img.naturalWidth || box) * k, (img.naturalHeight || box) * k];
      g.drawImage(img, x + (size - w) / 2, y + (size - h) / 2, w, h);
    } catch {
      /* draw the coloured tile only */
    }
  } else {
    g.font = `${size * 0.62}px ${EMOJI_FONT}`;
    g.textAlign = "center";
    g.textBaseline = "middle";
    g.fillStyle = "#fff";
    g.fillText(icon.value || "📈", x + size / 2, y + size / 2 + size * 0.04);
  }
}

/** Draws a share card for one number. */
export async function renderSnapshot(v: EntryView, now = Date.now()): Promise<HTMLCanvasElement> {
  await Promise.all(["800 72px", "600 20px", "400 13px"].map((f) => document.fonts?.load(`${f} "Inter Tight"`).catch(() => [])));
  const c = document.createElement("canvas");
  c.width = W * SCALE;
  c.height = H * SCALE;
  const g = c.getContext("2d");
  if (!g) throw new Error("drawing is not available in this window");
  g.scale(SCALE, SCALE);
  const info = shareDetails(v, now);
  const inner = W - PAD * 2;

  const bg = g.createLinearGradient(0, 0, W, H);
  bg.addColorStop(0, "#181a21");
  bg.addColorStop(1, "#0b0c10");
  rounded(g, 0, 0, W, H, 28);
  g.fillStyle = bg;
  g.fill();
  rounded(g, 0.5, 0.5, W - 1, H - 1, 28);
  g.strokeStyle = "rgba(255, 255, 255, 0.08)";
  g.lineWidth = 1;
  g.stroke();

  await drawIcon(g, v.entry.icon, PAD, PAD, 44);

  g.textAlign = "right";
  g.textBaseline = "alphabetic";
  g.font = `800 16px ${FONT}`;
  g.fillStyle = "#5b8cff";
  g.fillText("Cownter", W - PAD, PAD + 20);
  const mark = g.measureText("Cownter").width;

  g.textAlign = "left";
  g.font = `600 20px ${FONT}`;
  g.fillStyle = "#e8eaf0";
  g.fillText(ellipsize(v.entry.name, inner - 56 - mark - 20, (t) => g.measureText(t).width), PAD + 56, PAD + 20);
  g.font = `400 13px ${FONT}`;
  g.fillStyle = "#8a92a0";
  g.fillText(new Date(now).toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" }), PAD + 56, PAD + 40);

  let size = 76;
  g.font = `800 ${size}px ${FONT}`;
  while (size > 28 && g.measureText(info.value).width > inner) {
    size -= 4;
    g.font = `800 ${size}px ${FONT}`;
  }
  g.fillStyle = "#ffffff";
  g.fillText(ellipsize(info.value, inner, (t) => g.measureText(t).width), PAD, 176);

  let x = PAD;
  g.font = `600 20px ${FONT}`;
  if (info.delta) {
    g.fillStyle = info.up ? "#7ee2a8" : "#ff8a7a";
    g.fillText(info.delta, x, 212);
    x += g.measureText(info.delta).width + 18;
  }
  if (info.streak) {
    g.fillStyle = "#ffb35c";
    g.fillText(info.streak, x, 212);
  }

  const bottom = info.goal ? 300 : 336;
  const pts = sparkCoords(v.spark, PAD, 236, inner, bottom - 236);
  if (pts.length) {
    const color = info.up ? "#5b8cff" : "#ff6b6b";
    const fill = g.createLinearGradient(0, 236, 0, bottom);
    fill.addColorStop(0, info.up ? "rgba(91, 140, 255, 0.28)" : "rgba(255, 107, 107, 0.28)");
    fill.addColorStop(1, "rgba(0, 0, 0, 0)");
    g.beginPath();
    pts.forEach(([px, py], i) => (i ? g.lineTo(px, py) : g.moveTo(px, py)));
    g.lineTo(pts[pts.length - 1][0], bottom);
    g.lineTo(pts[0][0], bottom);
    g.closePath();
    g.fillStyle = fill;
    g.fill();
    g.beginPath();
    pts.forEach(([px, py], i) => (i ? g.lineTo(px, py) : g.moveTo(px, py)));
    g.strokeStyle = color;
    g.lineWidth = 2.5;
    g.lineCap = "round";
    g.lineJoin = "round";
    g.stroke();
  }

  if (info.goal) {
    g.font = `600 13px ${FONT}`;
    g.fillStyle = "#c9ced8";
    g.fillText(info.goal.label, PAD, 330);
    g.textAlign = "right";
    g.fillStyle = "#8a92a0";
    g.fillText(info.goal.left, W - PAD, 330);
    g.textAlign = "left";
    rounded(g, PAD, 340, inner, 6, 3);
    g.fillStyle = "rgba(255, 255, 255, 0.1)";
    g.fill();
    if (info.goal.progress > 0) {
      rounded(g, PAD, 340, Math.max(6, inner * info.goal.progress), 6, 3);
      g.fillStyle = "#5b8cff";
      g.fill();
    }
  }
  return c;
}

export function canvasBlob(c: HTMLCanvasElement): Promise<Blob> {
  return new Promise((resolve, reject) => c.toBlob((b) => (b ? resolve(b) : reject(new Error("the image could not be created"))), "image/png"));
}
