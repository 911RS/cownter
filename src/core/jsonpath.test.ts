import { describe, expect, it } from "vitest";
import { isNumericLeaf, joinPath } from "./jsonpath";

describe("joinPath", () => {
  it("joins keys and indexes", () => {
    expect(joinPath(["items", 0, "statistics", "subscriberCount"])).toBe("items[0].statistics.subscriberCount");
    expect(joinPath([0, "a"])).toBe("[0].a");
    expect(joinPath(["a.b", "c"])).toBe('["a.b"].c');
    expect(joinPath(['say "hi"'])).toBe('["say \\"hi\\""]');
  });
});

describe("isNumericLeaf", () => {
  it("accepts numbers and number-like strings", () => {
    for (const v of [42, 0.5, "1234", "1,234.50", "$12", "12 €", "45%", "1.2K", "-3"]) expect(isNumericLeaf(v)).toBe(true);
  });
  it("rejects everything else", () => {
    for (const v of [true, null, "hello", "", "abc123def", Number.POSITIVE_INFINITY, {}, []]) expect(isNumericLeaf(v)).toBe(false);
  });
});
