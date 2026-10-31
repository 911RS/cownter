/** Builds a path string understood by the Rust picker (`data.items[0]["a.b"]`). */
export function joinPath(parts: (string | number)[]): string {
  let out = "";
  for (const p of parts) {
    if (typeof p === "number") out += `[${p}]`;
    else if (/^[A-Za-z_$][\w$-]*$/.test(p)) out += (out ? "." : "") + p;
    else out += `["${p.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"]`;
  }
  return out;
}

/** True when a JSON leaf can be read as a number by the Rust parser. */
export function isNumericLeaf(v: unknown): boolean {
  if (typeof v === "number") return Number.isFinite(v);
  if (typeof v !== "string") return false;
  const s = v.trim();
  if (!/\d/.test(s) || s.length > 40) return false;
  return /^(?:[A-Z]{0,3}|[^\w\s]{0,2})\s?[-−+]?\s?[^\w\s]?\s?\d[\d.,\s'  ]*\s?[kKmMbB%]?\s?(?:[A-Z]{0,3}|[^\w\s]{0,2})$/.test(s);
}
