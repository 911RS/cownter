import type { SkinName, Status, Tint } from "../core/types";
import { DotMatrix } from "./DotMatrix";
import { fontSkin } from "./FontText";
import { skinOptClasses } from "./options";
import { Neon } from "./Neon";
import { SevenSeg } from "./SevenSeg";

const RENDER: Record<SkinName, React.ComponentType<{ text: string; height: number; opts?: Record<string, boolean> }>> = {
  clean: fontSkin("clean"),
  heavy: fontSkin("heavy"),
  light: fontSkin("light"),
  geometric: fontSkin("geometric"),
  modern: fontSkin("modern"),
  rounded: fontSkin("rounded"),
  tall: fontSkin("tall"),
  poster: fontSkin("poster"),
  wide: fontSkin("wide"),
  grotesk: fontSkin("grotesk"),
  future: fontSkin("future"),
  serif: fontSkin("serif"),
  elegant: fontSkin("elegant"),
  editorial: fontSkin("editorial"),
  mono: fontSkin("mono"),
  gradient: fontSkin("gradient"),
  outline: fontSkin("outline"),
  dotMatrix: DotMatrix,
  calculator: SevenSeg,
  neon: Neon,
};

interface Props {
  skin: SkinName;
  tint: Tint;
  text: string;
  height: number;
  status?: Status;
  opts?: Record<string, boolean>;
}

/** Draws a formatted number with the chosen skin. */
export function Skin({ skin, tint, text, height, status, opts }: Props) {
  const dim = status === "stale" || status === "error" || status === "noLink";
  const Render = RENDER[skin] ?? DotMatrix;
  return (
    <span className={`skin skin-${skin} tint-${tint}` + skinOptClasses(skin, opts) + (dim ? " skin-dim" : "")}>
      <Render text={text} height={height} opts={opts} />
    </span>
  );
}

export const TINTS: { id: Tint; label: string; swatch: string }[] = [
  { id: "white", label: "White", swatch: "#f4f6fa" },
  { id: "chroma", label: "RGB chroma", swatch: "conic-gradient(#ff4d4d, #ffd24d, #4dff88, #4dd2ff, #8a4dff, #ff4dd2, #ff4d4d)" },
  { id: "amber", label: "Amber", swatch: "#ffb340" },
  { id: "phosphor", label: "Phosphor", swatch: "#5dff8a" },
  { id: "ice", label: "Ice", swatch: "#5ad1ff" },
  { id: "ruby", label: "Ruby", swatch: "#ff5a5a" },
  { id: "violet", label: "Violet", swatch: "#b277ff" },
  { id: "classic", label: "Classic LCD", swatch: "#b9c1a2" },
];

export type SkinGroup = "Sans" | "Display" | "Serif" | "Mono" | "Effects" | "Retro";

export const SKINS: { id: SkinName; label: string; group: SkinGroup }[] = [
  { id: "clean", label: "Clean", group: "Sans" },
  { id: "heavy", label: "Heavy", group: "Sans" },
  { id: "light", label: "Light", group: "Sans" },
  { id: "geometric", label: "Geometric", group: "Sans" },
  { id: "modern", label: "Modern", group: "Sans" },
  { id: "rounded", label: "Rounded", group: "Sans" },
  { id: "tall", label: "Tall", group: "Display" },
  { id: "poster", label: "Poster", group: "Display" },
  { id: "wide", label: "Wide", group: "Display" },
  { id: "grotesk", label: "Grotesk", group: "Display" },
  { id: "future", label: "Future", group: "Display" },
  { id: "serif", label: "Serif", group: "Serif" },
  { id: "elegant", label: "Elegant", group: "Serif" },
  { id: "editorial", label: "Editorial", group: "Serif" },
  { id: "mono", label: "Mono", group: "Mono" },
  { id: "gradient", label: "Gradient", group: "Effects" },
  { id: "outline", label: "Outline", group: "Effects" },
  { id: "dotMatrix", label: "Dot-matrix", group: "Retro" },
  { id: "calculator", label: "Calculator", group: "Retro" },
  { id: "neon", label: "Neon", group: "Retro" },
];
