const SEG: Record<string, string> = {
  a: "2,0 8,0 9,1 8,2 2,2 1,1",
  b: "8,2 9,1 10,2 10,8 9,9 8,8",
  c: "8,10 9,9 10,10 10,16 9,17 8,16",
  d: "2,16 8,16 9,17 8,18 2,18 1,17",
  e: "0,10 1,9 2,10 2,16 1,17 0,16",
  f: "0,2 1,1 2,2 2,8 1,9 0,8",
  g: "2,8 8,8 9,9 8,10 2,10 1,9",
};

export const SEGMENTS: Record<string, string> = {
  "0": "abcdef",
  "1": "bc",
  "2": "abged",
  "3": "abgcd",
  "4": "fgbc",
  "5": "afgcd",
  "6": "afgedc",
  "7": "abc",
  "8": "abcdefg",
  "9": "abcdfg",
  "-": "g",
  "−": "g",
  "—": "g",
};

function Digit({ ch, h }: { ch: string; h: number }) {
  const lit = SEGMENTS[ch] ?? "";
  return (
    <svg className="ss-digit" viewBox="-1 -0.5 12 19" height={h} width={(h * 12) / 19} aria-hidden>
      <g transform="skewX(-6) translate(1 0)">
        {Object.entries(SEG).map(([k, pts]) => (
          <polygon key={k} points={pts} className={lit.includes(k) ? "ss-on" : "ss-off"} />
        ))}
      </g>
    </svg>
  );
}

export function SevenSeg({ text, height }: { text: string; height: number }) {
  const chars = [...text];
  return (
    <span className="ss" style={{ fontSize: height, padding: `${Math.max(2, height * 0.2)}px ${Math.max(3, height * 0.3)}px` }} role="img" aria-label={text}>
      {chars.map((c, i) =>
        SEGMENTS[c] !== undefined ? (
          <Digit key={i} ch={c} h={height} />
        ) : c === "." ? (
          <span key={i} className="ss-dot" />
        ) : c === "," ? (
          <span key={i} className="ss-dot ss-comma" />
        ) : c === " " ? (
          <span key={i} className="ss-space" />
        ) : (
          <span key={i} className="ss-sym">
            {c}
          </span>
        ),
      )}
    </span>
  );
}
