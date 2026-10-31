import { describe, expect, it } from "vitest";
import { isPack, packOf, resolveSound } from "./sounds";

describe("sound packs", () => {
  it("picks the entry pack, then the global one, then Soft", () => {
    expect(packOf("arcade", "cash")).toBe("arcade");
    expect(packOf(null, "cash")).toBe("cash");
    expect(packOf(undefined, undefined)).toBe("soft");
    expect(packOf("kazoo", "cowbell")).toBe("cowbell");
    expect(packOf("kazoo", "nope")).toBe("soft");
  });
  it("keeps the chosen sound in the Soft pack", () => {
    expect(resolveSound("change", "bubble", null, "soft")).toBe("bubble");
    expect(resolveSound("goal", "sparkle")).toBe("sparkle");
  });
  it("swaps in the pack voice for the role", () => {
    expect(resolveSound("change", "bubble", null, "cash")).toBe("cash-change");
    expect(resolveSound("goal", "sparkle", "cowbell", "soft")).toBe("cowbell-goal");
    expect(resolveSound("change", "chime", "soft", "arcade")).toBe("chime");
  });
  it("stays silent when no sound is chosen", () => {
    expect(resolveSound("change", null, "cheer", "cheer")).toBeNull();
    expect(resolveSound("goal", "", null, "arcade")).toBeNull();
  });
  it("knows the pack ids", () => {
    expect(isPack("cheer")).toBe(true);
    expect(isPack("bell")).toBe(false);
  });
});
