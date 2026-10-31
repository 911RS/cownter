/** Shows only the last four characters of a token. */
export function maskToken(token: string): string {
  if (token.length <= 4) return "•".repeat(token.length);
  return "•".repeat(12) + token.slice(-4);
}

function shellQuote(s: string): string {
  return "'" + s.replace(/'/g, `'\\''`) + "'";
}

/** A curl command that sets `entry` to `value` through the local push endpoint. */
export function curlExample(url: string, token: string, entry: string, value: number | string = 42): string {
  const body = JSON.stringify({ entry, value });
  return [
    `curl -X POST ${url}/v1/set`,
    `  -H ${shellQuote(`Authorization: Bearer ${token}`)}`,
    `  -H 'Content-Type: application/json'`,
    `  -d ${shellQuote(body)}`,
  ].join(" \\\n");
}

/** A port the endpoint can use, or null. */
export function parsePort(text: string): number | null {
  const n = Number(text.trim());
  return Number.isInteger(n) && n >= 1024 && n <= 65535 ? n : null;
}
