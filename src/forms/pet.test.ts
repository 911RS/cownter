import { describe, expect, it } from "vitest";
import { PET_STAGES, petStageIndex, petTitle } from "./pet";

describe("pet stages", () => {
  it("grows with progress to the goal", () => {
    expect(petStageIndex(0, 100)).toBe(0);
    expect(petStageIndex(9, 100)).toBe(0);
    expect(petStageIndex(10, 100)).toBe(1);
    expect(petStageIndex(40, 100)).toBe(2);
    expect(petStageIndex(70, 100)).toBe(3);
    expect(petStageIndex(99.9, 100)).toBe(3);
    expect(petStageIndex(100, 100)).toBe(4);
    expect(petStageIndex(250, 100)).toBe(4);
  });
  it("uses the number of digits without a goal", () => {
    expect(petStageIndex(0.5, null)).toBe(0);
    expect(petStageIndex(7, null)).toBe(0);
    expect(petStageIndex(42, null)).toBe(1);
    expect(petStageIndex(999, null)).toBe(2);
    expect(petStageIndex(1000, null)).toBe(3);
    expect(petStageIndex(1e7, null)).toBe(4);
    expect(petStageIndex(-500, null)).toBe(0);
    expect(petStageIndex(500, 0)).toBe(2);
  });
  it("starts as an egg without a value", () => {
    expect(petStageIndex(null, 100)).toBe(0);
    expect(petStageIndex(Number.NaN, null)).toBe(0);
  });
  it("names the stage on hover", () => {
    const entry = { name: "Stars", goalValue: 200, textMode: false } as never;
    expect(petTitle({ value: 150, entry })).toBe(`${PET_STAGES[3].label} · Stars · 75% of the goal`);
    expect(petTitle({ value: 5, entry: { name: "Subs", goalValue: null } as never })).toBe("Egg · Subs");
  });
});
