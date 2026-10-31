import type { EntryView, FormCfg } from "./types";

/** Entries a form should show, in display order. */
export function selectEntries(views: EntryView[], cfg: Pick<FormCfg, "groupId" | "entryIds">): EntryView[] {
  const sorted = [...views].sort((a, b) => a.entry.sort - b.entry.sort || a.entry.id - b.entry.id);
  if (cfg.entryIds.length > 0) {
    const set = new Set(cfg.entryIds);
    return sorted.filter((v) => set.has(v.entry.id));
  }
  if (cfg.groupId !== null) return sorted.filter((v) => v.entry.groupId === cfg.groupId);
  return sorted;
}

export const SIZE_SCALE: Record<string, number> = { xs: 0.7, s: 0.85, m: 1, l: 1.2, xl: 1.45 };
