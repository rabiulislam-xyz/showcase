import { describe, it, expect } from "vitest";
import { mainCategory, availableCategories } from "./categories";
import type { App } from "./types";

function makeApp(categories: string[]): App {
  return {
    uid: "apt:test",
    source: "apt",
    name: "Test",
    summary: null,
    description: null,
    version: null,
    icon_path: null,
    size_bytes: null,
    install_date: null,
    publisher: null,
    categories,
    exec: null,
    desktop_path: null,
    pkg_ref: "test",
    removable: true,
    protected_reason: null,
    update_available: null,
  };
}

describe("mainCategory", () => {
  it('maps Network → "Internet"', () => {
    expect(mainCategory(["Network", "WebBrowser"])).toBe("Internet");
  });

  it('maps AudioVideo → "Audio & Video"', () => {
    expect(mainCategory(["AudioVideo", "Player"])).toBe("Audio & Video");
  });

  it('maps Audio (without AudioVideo) → "Audio & Video"', () => {
    expect(mainCategory(["Audio"])).toBe("Audio & Video");
  });

  it('maps Utility → "Utilities"', () => {
    expect(mainCategory(["Utility"])).toBe("Utilities");
  });

  it('maps Game → "Games"', () => {
    expect(mainCategory(["Game"])).toBe("Games");
  });

  it('maps Development → "Development"', () => {
    expect(mainCategory(["Development"])).toBe("Development");
  });

  it('maps Graphics → "Graphics"', () => {
    expect(mainCategory(["Graphics"])).toBe("Graphics");
  });

  it('empty categories → "Other"', () => {
    expect(mainCategory([])).toBe("Other");
  });

  it('unrecognised tags → "Other"', () => {
    expect(mainCategory(["Bogus", "UnknownTag"])).toBe("Other");
  });

  // Priority: Game beats Utility when both are present
  it('Game takes priority over Utility', () => {
    expect(mainCategory(["Utility", "Game"])).toBe("Games");
  });
});

describe("availableCategories", () => {
  it("returns distinct groups sorted alphabetically with Other last", () => {
    const apps: App[] = [
      makeApp(["Network"]),       // Internet
      makeApp(["AudioVideo"]),    // Audio & Video
      makeApp(["Graphics"]),      // Graphics
      makeApp([]),                // Other
      makeApp(["Network"]),       // Internet (duplicate — deduped)
    ];
    const result = availableCategories(apps);
    // Alphabetical: Audio & Video, Graphics, Internet — then Other at the end
    expect(result).toEqual(["Audio & Video", "Graphics", "Internet", "Other"]);
  });

  it("omits Other when no apps map to it", () => {
    const apps: App[] = [makeApp(["Network"]), makeApp(["Graphics"])];
    const result = availableCategories(apps);
    expect(result).toEqual(["Graphics", "Internet"]);
  });

  it("returns empty array for an empty app list", () => {
    expect(availableCategories([])).toEqual([]);
  });

  it("returns only Other when all apps have unrecognised categories", () => {
    const apps: App[] = [makeApp(["Bogus"]), makeApp([])];
    expect(availableCategories(apps)).toEqual(["Other"]);
  });
});
