import type { Tint } from "./types";

export type Season = "halloween" | "christmas" | "newyear";

export const SEASONS: Record<Season, { label: string; tint: Tint; mark: string | null }> = {
  halloween: { label: "Halloween", tint: "pumpkin", mark: "🎃" },
  christmas: { label: "Christmas", tint: "frost", mark: "❄" },
  newyear: { label: "New Year", tint: "gold", mark: null },
};

/** Season for a local date: Halloween Oct 20 to Nov 1, Christmas Dec 10 to 26, New Year Dec 31 to Jan 2. */
export function seasonOf(d: Date): Season | null {
  const m = d.getMonth() + 1;
  const day = d.getDate();
  if ((m === 10 && day >= 20) || (m === 11 && day === 1)) return "halloween";
  if (m === 12 && day >= 10 && day <= 26) return "christmas";
  if ((m === 12 && day === 31) || (m === 1 && day <= 2)) return "newyear";
  return null;
}

/** Tint to draw with: the season's tint while one is active, else the form's own. */
export function seasonTint(season: Season | null, tint: Tint): Tint {
  return season ? SEASONS[season].tint : tint;
}

export function seasonClass(season: Season | null): string {
  return season ? " season-" + season : "";
}
