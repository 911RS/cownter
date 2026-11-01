import { beforeEach, describe, expect, it } from "vitest";
import { noteDelta, resetDeltas } from "./deltaHold";

describe("noteDelta", () => {
  beforeEach(() => resetDeltas());

  it("does not count the first value seen as a change", () => {
    expect(noteDelta(1, 5, 1000)).toBe(0);
    expect(noteDelta(1, 5, 2000)).toBe(0);
  });

  it("stamps the time when today's change moves", () => {
    noteDelta(1, 5, 1000);
    expect(noteDelta(1, 6, 2000)).toBe(2000);
    expect(noteDelta(1, 6, 9000)).toBe(2000);
  });

  it("ignores a change that appears from nothing", () => {
    noteDelta(1, null, 1000);
    expect(noteDelta(1, 0, 2000)).toBe(0);
  });

  it("tracks entries separately", () => {
    noteDelta(1, 1, 1000);
    noteDelta(2, 1, 1000);
    expect(noteDelta(1, 2, 3000)).toBe(3000);
    expect(noteDelta(2, 1, 3000)).toBe(0);
  });
});
