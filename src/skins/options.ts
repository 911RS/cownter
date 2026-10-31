import type { SkinName } from "../core/types";

export interface SkinOpt {
  id: string;
  label: string;
  def: boolean;
}

const TEXT: SkinOpt[] = [
  { id: "italic", label: "Italic", def: false },
  { id: "glow", label: "Soft glow", def: false },
  { id: "shadow", label: "Drop shadow", def: false },
  { id: "caps", label: "Small symbols", def: false },
];

export const SKIN_OPTS: Record<SkinName, SkinOpt[]> = {
  clean: TEXT,
  heavy: TEXT,
  light: TEXT,
  geometric: TEXT,
  modern: TEXT,
  rounded: TEXT,
  tall: TEXT,
  poster: TEXT,
  wide: TEXT,
  grotesk: TEXT,
  future: TEXT,
  serif: TEXT,
  elegant: TEXT,
  editorial: TEXT,
  mono: TEXT,
  gradient: TEXT,
  outline: TEXT,
  dotMatrix: [
    { id: "panel", label: "Dark panel behind the dots", def: false },
    { id: "unlit", label: "Show unlit dots", def: true },
    { id: "square", label: "Square dots", def: false },
    { id: "glow", label: "Glow", def: true },
  ],
  calculator: [
    { id: "panel", label: "LCD panel", def: false },
    { id: "ghost", label: "Unlit segments", def: true },
    { id: "glow", label: "Glow", def: true },
    { id: "italic", label: "Slanted", def: false },
  ],
  neon: [
    { id: "glow", label: "Glow", def: true },
    { id: "core", label: "Bright core", def: true },
    { id: "flicker", label: "Flicker on change", def: true },
  ],
};

export function skinOpt(skin: SkinName, opts: Record<string, boolean> | undefined, id: string): boolean {
  const v = opts?.[`${skin}.${id}`];
  if (typeof v === "boolean") return v;
  return SKIN_OPTS[skin]?.find((o) => o.id === id)?.def ?? false;
}

/** Class names for the options that are switched on, e.g. ` o-panel o-glow`. */
export function skinOptClasses(skin: SkinName, opts: Record<string, boolean> | undefined): string {
  return (SKIN_OPTS[skin] ?? [])
    .filter((o) => skinOpt(skin, opts, o.id))
    .map((o) => " o-" + o.id)
    .join("");
}
