import { describe, expect, it } from "vitest";
import { compact, etaDate, formatDelta, formatValue, goalLeft, intervalLabel, slugify, timeAgo } from "./format";

describe("compact", () => {
  it("shortens large numbers", () => {
    expect(compact(999)).toBe("999");
    expect(compact(1000)).toBe("1K");
    expect(compact(1250)).toBe("1.2K");
    expect(compact(9990)).toBe("9.9K");
    expect(compact(12_400)).toBe("12K");
    expect(compact(3_400_000)).toBe("3.4M");
    expect(compact(2_000_000_000)).toBe("2B");
    expect(compact(-1500)).toBe("-1.5K");
  });
});

describe("formatValue", () => {
  it("groups thousands and applies prefix and suffix", () => {
    expect(formatValue(1234567)).toBe("1,234,567");
    expect(formatValue(12.5, { prefix: "$" })).toBe("$12.50");
    expect(formatValue(42, { suffix: " subs" })).toBe("42 subs");
    expect(formatValue(1234.567, { decimals: 1 })).toBe("1,234.6");
    expect(formatValue(-1234)).toBe("-1,234");
  });
  it("uses compact style", () => {
    expect(formatValue(15300, { style: "compact" })).toBe("15K");
  });
  it("shows a dash for missing values", () => {
    expect(formatValue(null)).toBe("—");
    expect(formatValue(Number.NaN)).toBe("—");
  });
  it("never prints negative zero", () => {
    expect(formatValue(-0.001, { decimals: 0 })).toBe("0");
  });
});

describe("formatDelta", () => {
  it("signs changes and hides zero", () => {
    expect(formatDelta(5)).toBe("+5");
    expect(formatDelta(-3)).toBe("−3");
    expect(formatDelta(0)).toBe("");
    expect(formatDelta(25000)).toBe("+25K");
  });
});

describe("slugify", () => {
  it("makes formula-safe names", () => {
    expect(slugify("YouTube Subs")).toBe("youtube_subs");
    expect(slugify("Café orders!")).toBe("cafe_orders");
    expect(slugify("2024 sales")).toBe("n_2024_sales");
    expect(slugify("***")).toBe("n");
    expect(slugify("x".repeat(60)).length).toBeLessThanOrEqual(40);
  });
});

describe("timeAgo and intervalLabel", () => {
  it("describes elapsed time", () => {
    const now = 1_000_000_000;
    expect(timeAgo(null, now)).toBe("never");
    expect(timeAgo(now - 3000, now)).toBe("just now");
    expect(timeAgo(now - 30_000, now)).toBe("30s ago");
    expect(timeAgo(now - 5 * 60_000, now)).toBe("5 min ago");
    expect(timeAgo(now - 3 * 3600_000, now)).toBe("3 h ago");
    expect(timeAgo(now - 2 * 86400_000, now)).toBe("2 d ago");
  });
  it("labels poll intervals", () => {
    expect(intervalLabel(30)).toBe("30 s");
    expect(intervalLabel(300)).toBe("5 min");
    expect(intervalLabel(7200)).toBe("2 h");
  });
});

describe("goalLeft", () => {
  const fmt = { style: "full" as const, decimals: null, prefix: "", suffix: "" };
  const now = Date.UTC(2026, 9, 31, 12);
  const mk = (value: number | null, goalValue: number | null, goalEta: number | null = null) => ({ value, goalEta, entry: { goalValue, format: fmt } });

  it("shows what is left and the forecast date", () => {
    expect(goalLeft(mk(847, 1000), now)).toBe("153 to go");
    expect(goalLeft(mk(847, 1000, Date.UTC(2026, 10, 12, 12)), now)).toBe("153 to go · ~Nov 12");
    expect(goalLeft(mk(0, 50_000), now)).toBe("50K to go");
  });

  it("is empty without a goal, a value, or once reached", () => {
    expect(goalLeft(mk(5, null), now)).toBe("");
    expect(goalLeft(mk(null, 10), now)).toBe("");
    expect(goalLeft(mk(10, 10), now)).toBe("");
    expect(goalLeft({ ...mk(1, 10), entry: { goalValue: 10, format: fmt, textMode: true } }, now)).toBe("");
  });

  it("adds the year to far dates", () => {
    expect(etaDate(Date.UTC(2028, 2, 3, 12), now)).toBe("Mar 2028");
  });
});
