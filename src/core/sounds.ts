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
  }
}
