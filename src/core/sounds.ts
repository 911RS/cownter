export const SOUNDS = [
  { id: "chime", label: "Chime" },
  { id: "bubble", label: "Bubble" },
  { id: "tick", label: "Soft tick" },
  { id: "pop", label: "Pop" },
  { id: "marimba", label: "Marimba" },
  { id: "bell", label: "Glass bell" },
  { id: "sparkle", label: "Sparkle" },
] as const;

export type SoundId = (typeof SOUNDS)[number]["id"];

export const PACKS = [
  { id: "soft", label: "Soft" },
  { id: "cash", label: "Cash register" },
  { id: "cheer", label: "Crowd cheer" },
  { id: "arcade", label: "Arcade" },
  { id: "cowbell", label: "Cowbell" },
] as const;

export type PackId = (typeof PACKS)[number]["id"];
export type SoundRole = "change" | "goal";

/** Default sound for a role when a number turns a sound on. */
export const ROLE_DEFAULT: Record<SoundRole, SoundId> = { change: "chime", goal: "sparkle" };

export function isPack(id: string | null | undefined): id is PackId {
  return PACKS.some((p) => p.id === id);
}

/** Pack in effect: the number's own pick, else the global one, else Soft. */
export function packOf(entryPack: string | null | undefined, globalPack: string | null | undefined): PackId {
  if (isPack(entryPack)) return entryPack;
  if (isPack(globalPack)) return globalPack;
  return "soft";
}

/** Sound to play for a role. Soft keeps the chosen sound; other packs replace it with their own voice. */
export function resolveSound(role: SoundRole, chosen: string | null | undefined, entryPack?: string | null, globalPack?: string | null): string | null {
  if (!chosen) return null;
  const pack = packOf(entryPack, globalPack);
  return pack === "soft" ? chosen : `${pack}-${role}`;
}

let ctx: AudioContext | null = null;

function audio(): AudioContext | null {
  try {
    if (!ctx) ctx = new AudioContext();
    if (ctx.state === "suspended") void ctx.resume();
    return ctx;
  } catch {
    return null;
  }
}

interface ToneOpts {
  type?: OscillatorType;
  gain?: number;
  attack?: number;
  glideTo?: number;
  glideTime?: number;
}

function tone(c: AudioContext, out: AudioNode, freq: number, at: number, dur: number, o: ToneOpts = {}) {
  const osc = c.createOscillator();
  const g = c.createGain();
  osc.type = o.type ?? "sine";
  osc.frequency.setValueAtTime(freq, at);
  if (o.glideTo) osc.frequency.exponentialRampToValueAtTime(o.glideTo, at + (o.glideTime ?? dur * 0.6));
  const peak = o.gain ?? 0.2;
  g.gain.setValueAtTime(0.0001, at);
  g.gain.exponentialRampToValueAtTime(peak, at + (o.attack ?? 0.006));
  g.gain.exponentialRampToValueAtTime(0.0001, at + dur);
  osc.connect(g).connect(out);
  osc.start(at);
  osc.stop(at + dur + 0.05);
}

function noise(c: AudioContext, out: AudioNode, at: number, dur: number, freq: number, gain: number) {
  const len = Math.max(1, Math.floor(c.sampleRate * dur));
  const buf = c.createBuffer(1, len, c.sampleRate);
  const d = buf.getChannelData(0);
  for (let i = 0; i < len; i++) d[i] = (Math.random() * 2 - 1) * (1 - i / len) ** 3;
  const src = c.createBufferSource();
  src.buffer = buf;
  const bp = c.createBiquadFilter();
  bp.type = "bandpass";
  bp.frequency.value = freq;
  bp.Q.value = 1.4;
  const g = c.createGain();
  g.gain.value = gain;
  src.connect(bp).connect(g).connect(out);
  src.start(at);
}

function bell(c: AudioContext, out: AudioNode, freq: number, at: number, dur: number, gain: number) {
  const car = c.createOscillator();
  const mod = c.createOscillator();
  const modGain = c.createGain();
  const g = c.createGain();
  car.frequency.value = freq;
  mod.frequency.value = freq * 3.5;
  modGain.gain.setValueAtTime(freq * 2.2, at);
  modGain.gain.exponentialRampToValueAtTime(1, at + dur);
  mod.connect(modGain).connect(car.frequency);
  g.gain.setValueAtTime(0.0001, at);
  g.gain.exponentialRampToValueAtTime(gain, at + 0.008);
  g.gain.exponentialRampToValueAtTime(0.0001, at + dur);
  car.connect(g).connect(out);
  car.start(at);
  mod.start(at);
  car.stop(at + dur + 0.05);
  mod.stop(at + dur + 0.05);
}

function swell(c: AudioContext, out: AudioNode, at: number, dur: number, freq: number, gain: number, rise = 0.3) {
  const len = Math.max(1, Math.floor(c.sampleRate * dur));
  const buf = c.createBuffer(1, len, c.sampleRate);
  const d = buf.getChannelData(0);
  const up = Math.max(1, Math.floor(len * rise));
  for (let i = 0; i < len; i++) d[i] = (Math.random() * 2 - 1) * (i < up ? i / up : (1 - (i - up) / (len - up)) ** 2);
  const src = c.createBufferSource();
  src.buffer = buf;
  const bp = c.createBiquadFilter();
  bp.type = "bandpass";
  bp.frequency.value = freq;
  bp.Q.value = 0.7;
  const g = c.createGain();
  g.gain.value = gain;
  src.connect(bp).connect(g).connect(out);
  src.start(at);
}

function cowbell(c: AudioContext, out: AudioNode, at: number, gain: number, pitch = 1) {
  const bp = c.createBiquadFilter();
  bp.type = "bandpass";
  bp.frequency.value = 2640 * pitch;
  bp.Q.value = 1.2;
  const g = c.createGain();
  g.gain.setValueAtTime(0.0001, at);
  g.gain.exponentialRampToValueAtTime(gain, at + 0.004);
  g.gain.exponentialRampToValueAtTime(gain * 0.3, at + 0.06);
  g.gain.exponentialRampToValueAtTime(0.0001, at + 0.6);
  bp.connect(g).connect(out);
  for (const f of [540, 800]) {
    const o = c.createOscillator();
    o.type = "square";
    o.frequency.value = f * pitch;
    o.connect(bp);
    o.start(at);
    o.stop(at + 0.65);
  }
  tone(c, out, 800 * pitch, at, 0.25, { gain: gain * 0.25 });
}

/** Plays a built-in sound. `volume` is 0..1. */
export function playSound(id: string | null | undefined, volume = 0.6) {
  if (!id || volume <= 0) return;
  const c = audio();
  if (!c) return;
  const master = c.createGain();
  master.gain.value = Math.min(1, Math.max(0, volume));
  const lp = c.createBiquadFilter();
  lp.type = "lowpass";
  lp.frequency.value = 6000;
  master.connect(lp).connect(c.destination);
  const t = c.currentTime + 0.02;
  switch (id) {
    case "chime":
      tone(c, master, 880, t, 1.1, { gain: 0.16 });
      tone(c, master, 1318.5, t + 0.09, 1.3, { gain: 0.12 });
      break;
    case "bubble":
      tone(c, master, 320, t, 0.2, { gain: 0.22, glideTo: 920, glideTime: 0.11 });
      break;
    case "tick":
      noise(c, master, t, 0.035, 2200, 0.5);
      tone(c, master, 1800, t, 0.04, { gain: 0.05 });
      break;
    case "pop":
      tone(c, master, 700, t, 0.11, { gain: 0.25, glideTo: 180, glideTime: 0.08 });
      break;
    case "marimba":
      tone(c, master, 523.25, t, 0.45, { gain: 0.22 });
      tone(c, master, 2093, t, 0.08, { gain: 0.05 });
      tone(c, master, 783.99, t + 0.12, 0.55, { gain: 0.2 });
      tone(c, master, 3136, t + 0.12, 0.08, { gain: 0.04 });
      break;
    case "bell":
      bell(c, master, 659.25, t, 1.8, 0.14);
      break;
    case "sparkle":
      [523.25, 659.25, 783.99, 1046.5].forEach((f, i) => tone(c, master, f, t + i * 0.075, 0.6 + i * 0.1, { type: "triangle", gain: 0.13 }));
      tone(c, master, 2093, t + 0.32, 0.9, { gain: 0.05 });
      break;
    case "cash-change":
      noise(c, master, t, 0.05, 3200, 0.45);
      tone(c, master, 2093, t + 0.05, 0.5, { type: "triangle", gain: 0.12 });
      tone(c, master, 2637, t + 0.05, 0.7, { gain: 0.1 });
      break;
    case "cash-goal":
      noise(c, master, t, 0.08, 1400, 0.5);
      noise(c, master, t + 0.09, 0.05, 3200, 0.4);
      [2093, 2637, 3136, 2637, 3520, 4186].forEach((f, i) => tone(c, master, f, t + 0.14 + i * 0.06, 0.35, { type: "triangle", gain: 0.08 }));
      bell(c, master, 1318.5, t + 0.14, 1.4, 0.12);
      break;
    case "cheer-change":
      swell(c, master, t, 0.9, 1100, 0.55);
      tone(c, master, 520, t + 0.08, 0.5, { type: "triangle", gain: 0.04, glideTo: 880, glideTime: 0.3 });
      break;
    case "cheer-goal":
      swell(c, master, t, 2.2, 1000, 0.7, 0.25);
      swell(c, master, t + 0.1, 2, 2400, 0.25, 0.3);
      for (let i = 0; i < 9; i++) noise(c, master, t + 0.25 + i * 0.16 + (i % 3) * 0.03, 0.03, 1800, 0.5);
      [440, 554, 660].forEach((f, i) => tone(c, master, f, t + 0.2 + i * 0.12, 0.7, { type: "triangle", gain: 0.035, glideTo: f * 1.6, glideTime: 0.4 }));
      break;
    case "arcade-change":
      tone(c, master, 988, t, 0.07, { type: "square", gain: 0.08 });
      tone(c, master, 1319, t + 0.07, 0.22, { type: "square", gain: 0.08 });
      break;
    case "arcade-goal":
      [523.25, 659.25, 783.99, 1046.5, 783.99, 1046.5].forEach((f, i) => tone(c, master, f, t + i * 0.08, 0.1, { type: "square", gain: 0.07 }));
      tone(c, master, 1318.5, t + 0.48, 0.5, { type: "square", gain: 0.07 });
      tone(c, master, 659.25, t + 0.48, 0.5, { type: "triangle", gain: 0.1 });
      break;
    case "cowbell-change":
      cowbell(c, master, t, 0.22);
      break;
    case "cowbell-goal":
      cowbell(c, master, t, 0.2);
      cowbell(c, master, t + 0.18, 0.2);
      cowbell(c, master, t + 0.36, 0.24, 1.12);
      break;
  }
}
