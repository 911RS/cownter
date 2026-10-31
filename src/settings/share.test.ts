import { describe, expect, it } from "vitest";
import type { EntryView } from "../core/types";
import { ellipsize, shareDetails, sparkCoords } from "./share";

function view(patch: Partial<EntryView> = {}, entry: Partial<EntryView["entry"]> = {}): EntryView {
  return {
    entry: {
      id: 1,
      slug: "stars",
      name: "Stars",
      icon: { kind: "emoji", value: "⭐", bg: "#000000" },
      groupId: null,
      source: { type: "formula", expr: "1" },
      intervalS: 60,
      format: { style: "full", decimals: null, prefix: "", suffix: "" },
      goalValue: null,
      celebrate: false,
      sounds: { onChange: null, onGoal: null },
      sort: 0,
      ...entry,
    },
    value: 1234,
    prev: 1200,
    status: "live",
    error: null,
    updatedAt: 0,
    deltaToday: 34,
    spark: [],
    goalProgress: null,
    streak: 0,
    ...patch,
  };
}

describe("shareDetails", () => {
  it("lists value, delta and streak", () => {
    const d = shareDetails(view({ streak: 4 }));
    expect(d.value).toBe("1,234");
    expect(d.delta).toBe("+34 today");
    expect(d.up).toBe(true);
    expect(d.streak).toBe("🔥 4-day streak");
    expect(d.goal).toBeNull();
  });
  it("hides an empty delta and a short streak", () => {
    const d = shareDetails(view({ deltaToday: -3, streak: 1 }));
    expect(d.delta).toBe("−3 today");
    expect(d.up).toBe(false);
    expect(d.streak).toBe("");
    expect(shareDetails(view({ deltaToday: 0 })).delta).toBe("");
  });
  it("describes the goal", () => {
    const d = shareDetails(view({ goalProgress: 0.6, value: 600 }, { goalValue: 1000 }));
    expect(d.goal).toEqual({ progress: 0.6, label: "Goal 1,000 · 60%", left: "400 to go" });
    expect(shareDetails(view({ goalProgress: 1, value: 1000 }, { goalValue: 1000 })).goal?.left).toBe("Reached");
  });
});

describe("sparkCoords", () => {
  it("fits points into the box", () => {
    expect(sparkCoords([[0, 1]], 0, 0, 100, 50)).toEqual([]);
    expect(sparkCoords([[0, 0], [5, 10], [10, 5]], 10, 20, 100, 50)).toEqual([[10, 70], [60, 20], [110, 45]]);
    expect(sparkCoords([[0, 3], [10, 3]], 0, 0, 100, 50)).toEqual([[0, 25], [100, 25]]);
  });
});

describe("ellipsize", () => {
  const measure = (t: string) => [...t].length * 10;
  it("cuts long text", () => {
    expect(ellipsize("short", 100, measure)).toBe("short");
    expect(ellipsize("a long name here", 60, measure)).toBe("a lon…");
  });
});
