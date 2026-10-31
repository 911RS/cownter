import { describe, expect, it } from "vitest";
import { curlExample, maskToken, parsePort } from "./push";

describe("push helpers", () => {
  it("masks all but the end of the token", () => {
    expect(maskToken("abcdefghijklmnop")).toBe("••••••••••••mnop");
    expect(maskToken("abc")).toBe("•••");
  });

  it("builds a curl command with safe quoting", () => {
    const c = curlExample("http://127.0.0.1:47615", "tok", "Bob's orders");
    expect(c).toContain("curl -X POST http://127.0.0.1:47615/v1/set");
    expect(c).toContain("-H 'Authorization: Bearer tok'");
    expect(c).toContain(`-d '{"entry":"Bob'\\''s orders","value":42}'`);
  });

  it("accepts only unprivileged ports", () => {
    expect(parsePort("47615")).toBe(47615);
    expect(parsePort(" 2000 ")).toBe(2000);
    expect(parsePort("80")).toBeNull();
    expect(parsePort("70000")).toBeNull();
    expect(parsePort("12.5")).toBeNull();
    expect(parsePort("")).toBeNull();
  });
});
