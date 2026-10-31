import type { FormCfg } from "../core/types";

export interface FormOpt {
  id: string;
  label: string;
  def: boolean;
}

const COMMON: FormOpt[] = [
  { id: "status", label: "Status marks (stale, error)", def: true },
  { id: "pulse", label: "Animate the number when it changes", def: true },
];

export const FORM_OPTS: Record<string, FormOpt[]> = {
  ticker: [
    { id: "motion", label: "Scrolling", def: true },
    { id: "sep", label: "Separators", def: true },
  ],
  chips: [
    { id: "paper", label: "Pastel paper colours (off uses your background)", def: true },
    { id: "screen", label: "Dark box behind the number", def: true },
  ],
  trend: [{ id: "spark", label: "Sparklines", def: true }],
  bar: [{ id: "div", label: "Dividers", def: true }],
  deck: [{ id: "div", label: "Dividers", def: true }],
  versus: [
    { id: "vsbadge", label: "VS badge", def: true },
    { id: "vsbar", label: "Comparison bar", def: true },
  ],
  tag: [{ id: "string", label: "String and hole", def: true }],
  clothesline: [{ id: "rope", label: "Rope and pegs", def: true }],
  train: [
    { id: "travel", label: "Drive across the screen", def: true },
    { id: "motion", label: "Running animation", def: true },
    { id: "smoke", label: "Smoke", def: true },
  ],
  pet: [{ id: "bubble", label: "Speech bubble", def: true }],
  pager: [
    { id: "led", label: "Message light", def: true },
    { id: "time", label: "Updated time", def: true },
    { id: "grille", label: "Speaker grille", def: true },
    { id: "btn", label: "Side button", def: true },
  ],
  feed: [{ id: "time", label: "Times", def: true }],
};

export function formOptsFor(id: string): FormOpt[] {
  return [...(FORM_OPTS[id] ?? []), ...COMMON];
}

export function formOpt(id: string, f: FormCfg, opt: string): boolean {
  const v = f.formOpts?.[opt];
  if (typeof v === "boolean") return v;
  return formOptsFor(id).find((o) => o.id === opt)?.def ?? true;
}

/** Class names for the options that are switched off, e.g. ` fo-off-sep`. */
export function formOptClasses(id: string, f: FormCfg): string {
  return formOptsFor(id)
    .filter((o) => !formOpt(id, f, o.id))
    .map((o) => " fo-off-" + o.id)
    .join("");
}
