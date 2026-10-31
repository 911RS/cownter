import type { EntryView } from "../core/types";

export const PET_STAGES = [
  { id: "egg", label: "Egg" },
  { id: "baby", label: "Baby cow" },
  { id: "young", label: "Young cow" },
  { id: "adult", label: "Grown cow" },
  { id: "crowned", label: "Crowned cow" },
] as const;

export type PetStage = (typeof PET_STAGES)[number]["id"];

const GOAL_STEPS = [0.1, 0.35, 0.7, 1];

/** Stage index 0..4: progress to the goal when there is one, else the number of digits. */
export function petStageIndex(value: number | null, goal: number | null, textMode = false): number {
  if (textMode) return 3;
  if (value === null || !Number.isFinite(value)) return 0;
  if (goal !== null && goal > 0) {
    const p = value / goal;
    return GOAL_STEPS.filter((s) => p >= s).length;
  }
  if (value < 1) return 0;
  return Math.min(4, Math.floor(Math.log10(value)));
}

export function petStage(v: Pick<EntryView, "value" | "entry">): number {
  return petStageIndex(v.value, v.entry.goalValue, !!v.entry.textMode);
}

/** Hover text: stage name and how far along it is. */
export function petTitle(v: Pick<EntryView, "value" | "entry">): string {
  const i = petStage(v);
  const name = `${PET_STAGES[i].label} · ${v.entry.name}`;
  const g = v.entry.goalValue;
  if (v.entry.textMode || v.value === null || g === null || g <= 0) return name;
  return `${name} · ${Math.min(100, Math.round((v.value / g) * 100))}% of the goal`;
}
