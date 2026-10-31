export function Spark({ points, width = 80, height = 20, color = "currentColor" }: { points: [number, number][]; width?: number; height?: number; color?: string }) {
  if (points.length < 2) return <svg width={width} height={height} />;
  const xs = points.map((p) => p[0]);
  const ys = points.map((p) => p[1]);
  const [x0, x1] = [Math.min(...xs), Math.max(...xs)];
  const [y0, y1] = [Math.min(...ys), Math.max(...ys)];
  const sx = (x: number) => (x1 === x0 ? width / 2 : ((x - x0) / (x1 - x0)) * (width - 2) + 1);
  const sy = (y: number) => (y1 === y0 ? height / 2 : height - 2 - ((y - y0) / (y1 - y0)) * (height - 4));
  const d = points.map((p, i) => `${i ? "L" : "M"}${sx(p[0]).toFixed(1)} ${sy(p[1]).toFixed(1)}`).join("");
  return (
    <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`} aria-hidden>
      <path d={d} fill="none" stroke={color} strokeWidth={1.6} strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}
