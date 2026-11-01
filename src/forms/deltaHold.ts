import { createContext } from "react";
import type { EntryView } from "../core/types";

/** Seconds the change next to a number stays after it moves; 0 keeps it all day. */
export const DeltaHold = createContext(0);

const seen = new Map<number, { d: number | null; at: number }>();

/** Records today's change for each entry and returns when it last moved (0 if not seen moving). */
export function noteDelta(id: number, d: number | null, now = Date.now()): number {
  const s = seen.get(id);
  if (!s) {
    seen.set(id, { d, at: 0 });
    return 0;
  }
  if (s.d !== d) {
    const at = s.d === null ? 0 : now;
    seen.set(id, { d, at });
    return at;
  }
  return s.at;
}

export function noteDeltas(views: EntryView[], now = Date.now()) {
  for (const v of views) noteDelta(v.entry.id, v.deltaToday, now);
}

export function resetDeltas() {
  seen.clear();
}
