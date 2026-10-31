import type { SkinName, Status, Tint } from "../core/types";
import { DotMatrix } from "./DotMatrix";
import { fontSkin } from "./FontText";
import { skinOptClasses } from "./options";
import { Neon } from "./Neon";
import { SevenSeg } from "./SevenSeg";

const RENDER: Record<SkinName, React.ComponentType<{ text: string; height: number; opts?: Record<string, boolean> }>> = {
  clean: fontSkin("clean"),
  dmsans: fontSkin("dmsans"),
  manrope: fontSkin("manrope"),
  poppins: fontSkin("poppins"),
  montserrat: fontSkin("montserrat"),
  rubik: fontSkin("rubik"),
  lexend: fontSkin("lexend"),
  geometric: fontSkin("geometric"),
  modern: fontSkin("modern"),
  rounded: fontSkin("rounded"),
  grotesk: fontSkin("grotesk"),
  tall: fontSkin("tall"),
  barlow: fontSkin("barlow"),
  poster: fontSkin("poster"),
  wide: fontSkin("wide"),
  future: fontSkin("future"),
  mono: fontSkin("mono"),
  plexmono: fontSkin("plexmono"),
  spacemono: fontSkin("spacemono"),
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

export type SkinGroup = "Sans" | "Condensed" | "Wide" | "Mono" | "Retro";

export const SKINS: { id: SkinName; label: string; group: SkinGroup }[] = [
  { id: "clean", label: "Inter", group: "Sans" },
  { id: "dmsans", label: "DM Sans", group: "Sans" },
  { id: "manrope", label: "Manrope", group: "Sans" },
  { id: "poppins", label: "Poppins", group: "Sans" },
  { id: "montserrat", label: "Montserrat", group: "Sans" },
  { id: "rubik", label: "Rubik", group: "Sans" },
  { id: "lexend", label: "Lexend", group: "Sans" },
  { id: "geometric", label: "Outfit", group: "Sans" },
  { id: "modern", label: "Sora", group: "Sans" },
  { id: "rounded", label: "Nunito", group: "Sans" },
  { id: "grotesk", label: "Space Grotesk", group: "Sans" },
  { id: "tall", label: "Oswald", group: "Condensed" },
  { id: "barlow", label: "Barlow Condensed", group: "Condensed" },
  { id: "poster", label: "Bebas Neue", group: "Condensed" },
  { id: "wide", label: "Unbounded", group: "Wide" },
  { id: "future", label: "Orbitron", group: "Wide" },
  { id: "mono", label: "JetBrains Mono", group: "Mono" },
  { id: "plexmono", label: "IBM Plex Mono", group: "Mono" },
  { id: "spacemono", label: "Space Mono", group: "Mono" },
  { id: "dotMatrix", label: "Dot-matrix", group: "Retro" },
  { id: "calculator", label: "Calculator", group: "Retro" },
  { id: "neon", label: "Neon", group: "Retro" },
];
