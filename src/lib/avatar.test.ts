import { describe, it, expect } from "vitest";
import { tileColor, tileInitial, sourceLabel } from "./avatar";
import type { Source } from "./types";

// Mirrors the private TILE_PALETTE in avatar.ts. If the palette changes there,
// this test should fail and be updated deliberately.
const PALETTE = [
  "#D97757",
  "#B07A2B",
  "#4E837F",
  "#8E5A85",
  "#6A9BCC",
  "#788C5D",
];

describe("tileColor", () => {
  it("is deterministic for the same name", () => {
    expect(tileColor("Firefox")).toBe(tileColor("Firefox"));
    expect(tileColor("GIMP")).toBe(tileColor("GIMP"));
  });

  it("always returns a color from the known palette", () => {
    const names = ["Firefox", "GIMP", "VLC", "Discord", "OBS", "", "a", "zzzz", "Système"];
    for (const name of names) {
      expect(PALETTE).toContain(tileColor(name));
    }
  });

  it("returns the empty-string color (palette index 0) for an empty name", () => {
    // hash starts at 0, loop body never runs → index 0.
    expect(tileColor("")).toBe(PALETTE[0]);
  });

  it("distributes across more than one palette color", () => {
    const names = ["alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel"];
    const distinct = new Set(names.map(tileColor));
    expect(distinct.size).toBeGreaterThan(1);
  });
});

describe("tileInitial", () => {
  it("returns the first letter uppercased", () => {
    expect(tileInitial("firefox")).toBe("F");
    expect(tileInitial("GIMP")).toBe("G");
  });

  it("returns '?' for an empty string", () => {
    expect(tileInitial("")).toBe("?");
  });

  it("returns '?' for whitespace-only input", () => {
    expect(tileInitial("   ")).toBe("?");
    expect(tileInitial("\t\n")).toBe("?");
  });

  it("trims leading whitespace before taking the first letter", () => {
    expect(tileInitial("  spotify")).toBe("S");
  });
});

describe("sourceLabel", () => {
  it("maps each source to its display label", () => {
    const cases: Array<[Source, string]> = [
      ["apt", "APT"],
      ["flatpak", "Flatpak"],
      ["snap", "Snap"],
      ["appimage", "AppImage"],
    ];
    for (const [source, label] of cases) {
      expect(sourceLabel(source)).toBe(label);
    }
  });
});

describe("tileColor — appimage palette stability", () => {
  it("returns a defined, stable color for a name starting with 'A' (appimage territory)", () => {
    // The palette is deterministic: same name must always produce the same entry.
    const color = tileColor("AppImage");
    expect(PALETTE).toContain(color);
    // Stable across calls — not random.
    expect(tileColor("AppImage")).toBe(color);
  });
});
