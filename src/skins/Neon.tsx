/** Single-stroke tube paths in a 10 x 16 box. */
const TUBES: Record<string, { d: string; w: number }> = {
  "0": { d: "M5 1C2 1 1.2 4 1.2 8S2 15 5 15S8.8 12 8.8 8S8 1 5 1Z", w: 10 },
  "1": { d: "M2.6 4.2L5.6 1V15", w: 8 },
  "2": { d: "M1.4 4.6C1.4 2.3 3 1 5 1S8.6 2.3 8.6 4.6C8.6 8 1.4 11 1.4 15H8.8", w: 10 },
  "3": { d: "M1.6 1.4H8.2L4.6 6.8C7.2 6.8 8.8 8.4 8.8 10.9S7 15 4.9 15S1.7 14 1.2 12.5", w: 10 },
  "4": { d: "M7 15V1L1.2 10.8H9.4", w: 10.4 },
  "5": { d: "M8.4 1.2H2.6L2 7.2C2.9 6.5 3.9 6.1 5 6.1C7.4 6.1 8.8 8.1 8.8 10.6S7.1 15 4.9 15S1.6 14 1.2 12.6", w: 10 },
  "6": { d: "M7.8 2.1C7 1.4 6 1 5 1C2.6 1 1.2 3.9 1.2 8.8C1.2 13 2.8 15 5 15S8.8 13.3 8.8 10.6S7.2 6.3 5 6.3S1.7 7.6 1.2 9.6", w: 10 },
  "7": { d: "M1.2 1.2H8.8L4 15", w: 10 },
  "8": { d: "M5 7.6C2.9 7.6 1.6 6.4 1.6 4.3S3 1 5 1S8.4 2.2 8.4 4.3S7.1 7.6 5 7.6C2.5 7.6 1.2 9.1 1.2 11.3S2.8 15 5 15S8.8 13.6 8.8 11.3S7.5 7.6 5 7.6Z", w: 10 },
  "9": { d: "M8.8 6.4C8.4 8.6 7 9.8 5 9.8S1.2 8.3 1.2 5.4S2.9 1 5 1S8.8 2.9 8.8 7.2C8.8 12 7.4 15 5 15C4 15 3 14.7 2.2 14", w: 10 },
  "-": { d: "M1.8 9H7.2", w: 9 },
  "−": { d: "M1.8 9H7.2", w: 9 },
  "—": { d: "M1.4 9H8.6", w: 10 },
  "+": { d: "M1.8 9H8.2M5 5.8V12.2", w: 10 },
  ".": { d: "M2 14.6V14.7", w: 4 },
  ",": { d: "M2.4 13.8L1.4 16.4", w: 4 },
  " ": { d: "", w: 5 },
};

export function Neon({ text, height }: { text: string; height: number }) {
  const chars = [...text];
  const k = height / 16;
  return (
    <span className="neon" role="img" aria-label={text} key={text} style={{ fontSize: height * 1.1 }}>
      {chars.map((c, i) => {
        const t = TUBES[c];
        if (!t) return <span key={i} className="neon-sym">{c}</span>;
        return (
          <svg key={i} className="neon-ch" viewBox={`0 0 ${t.w} 17`} width={t.w * k} height={17 * k} aria-hidden>
            <path className="neon-glow" d={t.d} />
            <path className="neon-tube" d={t.d} />
            <path className="neon-core" d={t.d} />
          </svg>
        );
      })}
    </span>
  );
}
