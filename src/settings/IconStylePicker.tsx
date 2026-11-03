import { EntryIcon } from "../components/EntryIcon";
import type { EntryView, FormCfg, IconStyle } from "../core/types";
import { iconStyleOf, useShuffleTurn, valueText } from "../forms/common";

const STYLES: { id: IconStyle; label: string }[] = [
  { id: "tile", label: "Tile" },
  { id: "big", label: "Big" },
  { id: "glow", label: "Glow" },
  { id: "sticker", label: "Sticker" },
  { id: "glyph", label: "Glyph" },
  { id: "bar", label: "Bar" },
  { id: "fade", label: "Fade" },
  { id: "split", label: "Split" },
  { id: "cover", label: "Cover" },
  { id: "slant", label: "Slant" },
  { id: "ghost", label: "Ghost" },
  { id: "tilt", label: "Tilt" },
  { id: "wash", label: "Wash" },
  { id: "kinetic", label: "Kinetic" },
  { id: "shuffle", label: "Shuffle" },
];

const FALLBACK = { kind: "builtin", value: "star", bg: "#24292f" } as const;
const SHUFFLE = { iconStyle: "shuffle", cycleS: 2 } as FormCfg;

/** Icon style choices, each drawn as a small notch with the form's first number. */
export function IconStylePicker({ value, view, onChange }: { value: IconStyle; view: EntryView | null; onChange: (s: IconStyle) => void }) {
  const turn = useShuffleTurn(SHUFFLE);
  const icon = view?.entry.icon ?? FALLBACK;
  const num = view ? valueText(view) : "1,859";
  return (
    <div className="isp" role="radiogroup" aria-label="Icon style">
      {STYLES.map((o) => (
        <button type="button" role="radio" aria-checked={o.id === value} key={o.id} className={"isp-opt" + (o.id === value ? " on" : "")} onClick={() => onChange(o.id)}>
          <span className={"isp-demo is-" + (o.id === "shuffle" ? iconStyleOf(SHUFFLE, turn) : o.id)}>
            <span className="b-panel isp-notch">
              <span className="b-line">
                <EntryIcon icon={icon} size={16} />
                <span className="isp-num">{num}</span>
              </span>
            </span>
          </span>
          <span className="isp-label">{o.label}</span>
        </button>
      ))}
    </div>
  );
}
