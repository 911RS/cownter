export type FontStyle = "clean" | "dmsans" | "manrope" | "poppins" | "montserrat" | "rubik" | "lexend" | "geometric" | "modern" | "rounded" | "grotesk" | "tall" | "barlow" | "poster" | "wide" | "future" | "mono" | "plexmono" | "spacemono";

const SIZE: Partial<Record<FontStyle, number>> = { poppins: 0.96, montserrat: 0.94, lexend: 0.96, modern: 0.95, barlow: 1.1, poster: 1.22, wide: 0.86, future: 0.9 };

/** Draws the text in a regular typeface, sized so its digits are `height` pixels tall. */
export function FontText({ text, height, style }: { text: string; height: number; style: FontStyle }) {
  const parts = text.split(/([^0-9.,\-+\s]+)/).filter(Boolean);
  return (
    <span className={`ft ft-${style}`} style={{ fontSize: (height / 0.72) * (SIZE[style] ?? 1), ["--fth" as string]: `${height * 1.12}px` }}>
      {parts.map((p, i) => (/^[0-9.,\-+\s]+$/.test(p) ? p : <span className="ft-sym" key={i}>{p}</span>))}
    </span>
  );
}

export const fontSkin = (style: FontStyle) =>
  function Font({ text, height }: { text: string; height: number }) {
    return <FontText text={text} height={height} style={style} />;
  };
