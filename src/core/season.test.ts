import { describe, expect, it } from "vitest";
import { seasonClass, seasonOf, seasonTint } from "./season";

const at = (y: number, m: number, d: number, h = 12) => new Date(y, m - 1, d, h);

describe("seasonOf", () => {
  it("finds Halloween from Oct 20 to Nov 1", () => {
    expect(seasonOf(at(2026, 10, 19, 23))).toBeNull();
    expect(seasonOf(at(2026, 10, 20, 0))).toBe("halloween");
    expect(seasonOf(at(2026, 10, 31))).toBe("halloween");
    expect(seasonOf(at(2026, 11, 1, 23))).toBe("halloween");
    expect(seasonOf(at(2026, 11, 2, 0))).toBeNull();
  });
  it("finds Christmas from Dec 10 to 26", () => {
    expect(seasonOf(at(2026, 12, 9))).toBeNull();
    expect(seasonOf(at(2026, 12, 10))).toBe("christmas");
    expect(seasonOf(at(2026, 12, 26))).toBe("christmas");
    expect(seasonOf(at(2026, 12, 27))).toBeNull();
    expect(seasonOf(at(2026, 12, 30))).toBeNull();
  });
  it("finds New Year from Dec 31 to Jan 2", () => {
    expect(seasonOf(at(2026, 12, 31))).toBe("newyear");
    expect(seasonOf(at(2027, 1, 1))).toBe("newyear");
    expect(seasonOf(at(2027, 1, 2, 23))).toBe("newyear");
    expect(seasonOf(at(2027, 1, 3))).toBeNull();
  });
  it("is quiet the rest of the year", () => {
    for (const [m, d] of [[2, 14], [6, 1], [9, 30], [11, 15]]) expect(seasonOf(at(2026, m, d))).toBeNull();
  });
});

describe("season look", () => {
  it("overrides the tint only during a season", () => {
    expect(seasonTint("halloween", "ice")).toBe("pumpkin");
    expect(seasonTint("christmas", "white")).toBe("frost");
    expect(seasonTint("newyear", "white")).toBe("gold");
    expect(seasonTint(null, "ice")).toBe("ice");
  });
  it("adds a root class", () => {
    expect(seasonClass("christmas")).toBe(" season-christmas");
    expect(seasonClass(null)).toBe("");
  });
});
