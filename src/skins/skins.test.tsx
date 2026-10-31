import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { compact, formatDelta, formatValue } from "../core/format";
import { DotMatrix } from "./DotMatrix";
import { GLYPHS, glyphFor } from "./glyphs";

describe("glyphs", () => {
  it("are 7 rows of equal width", () => {
    for (const [ch, rows] of Object.entries(GLYPHS)) {
      expect(rows, ch).toHaveLength(7);
      expect(new Set(rows.map((r) => r.length)).size, ch).toBe(1);
    }
  });
  it("cover every character the formatter can produce", () => {
    const samples = [formatValue(-1234567.89), formatValue(9, { prefix: "$", suffix: "%" }), compact(3_400_000), compact(2e9), compact(1500), formatDelta(-5), formatValue(null), "€£¥"];
    for (const s of samples) for (const ch of s) expect(glyphFor(ch), JSON.stringify(ch)).not.toBe(GLYPHS["?"]);
  });
  it("falls back for unknown characters", () => {
    expect(glyphFor("Ω")).toBe(GLYPHS["?"]);
  });
});

describe("DotMatrix", () => {
  it("renders an accessible svg sized by height", () => {
    const { container } = render(<DotMatrix text="12" height={14} />);
    const svg = container.querySelector("svg")!;
    expect(svg.getAttribute("aria-label")).toBe("12");
    expect(svg.getAttribute("height")).toBe("14");
    expect(Number(svg.getAttribute("width"))).toBeCloseTo((14 * 110) / 70);
  });
});

describe("Skin", () => {
  it("renders every look with the text", async () => {
    const { Skin, SKINS } = await import("./Skin");
    for (const sk of SKINS) {
      const { container, unmount } = render(<Skin skin={sk.id} tint="white" text="4,821" height={20} />);
      expect(container.querySelector(".skin-" + sk.id), sk.id).not.toBeNull();
      expect(container.innerHTML.length, sk.id).toBeGreaterThan(20);
      unmount();
    }
  });
  it("draws text fonts as plain text", async () => {
    const { Skin } = await import("./Skin");
    const { container } = render(<Skin skin="serif" tint="white" text="1,234" height={20} />);
    expect(container.querySelector(".ft-serif")?.textContent).toBe("1,234");
  });
});

describe("skin options", () => {
  it("uses defaults and saved values", async () => {
    const { skinOpt, skinOptClasses } = await import("./options");
    expect(skinOpt("dotMatrix", undefined, "unlit")).toBe(true);
    expect(skinOpt("dotMatrix", undefined, "panel")).toBe(false);
    expect(skinOpt("dotMatrix", { "dotMatrix.panel": true }, "panel")).toBe(true);
    expect(skinOptClasses("neon", { "neon.flicker": false })).toBe(" o-glow o-core");
  });
});
