import { describe, expect, it } from "vitest";
import { selectEntries } from "./select";
import type { EntryView } from "./types";

const view = (id: number, sort: number, groupId: number | null) => ({ entry: { id, sort, groupId } }) as unknown as EntryView;
const views = [view(1, 2, null), view(2, 0, 7), view(3, 1, 7)];

describe("selectEntries", () => {
  it("returns everything sorted when nothing is picked", () => {
    expect(selectEntries(views, { groupId: null, entryIds: [] }).map((v) => v.entry.id)).toEqual([2, 3, 1]);
  });
  it("filters by group", () => {
    expect(selectEntries(views, { groupId: 7, entryIds: [] }).map((v) => v.entry.id)).toEqual([2, 3]);
  });
  it("explicit picks win over the group", () => {
    expect(selectEntries(views, { groupId: 7, entryIds: [1] }).map((v) => v.entry.id)).toEqual([1]);
  });
});

describe("text values", () => {
  it("shows the text and shortens long text", async () => {
    const { valueText } = await import("../forms/common");
    const base = { value: null, prev: null, status: "live", error: null, updatedAt: 1, deltaToday: null, spark: [], goalProgress: null } as const;
    const entry = { id: 1, slug: "s", name: "S", icon: { kind: "emoji", value: "x", bg: "#000000" }, groupId: null, source: { type: "formula", expr: "1" }, intervalS: 60, format: { style: "full", decimals: null, prefix: "", suffix: "" }, goalValue: null, celebrate: true, sounds: { onChange: null, onGoal: null }, sort: 0, textMode: true } as never;
    expect(valueText({ ...base, entry, text: "Shipped" } as never)).toBe("Shipped");
    expect(valueText({ ...base, entry, text: "x".repeat(40) } as never)).toHaveLength(28);
    expect(valueText({ ...base, entry, text: null, status: "polling" } as never)).toBe("...");
  });
});
