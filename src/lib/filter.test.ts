import { describe, it, expect } from "vitest";
import { filterAndSort } from "./filter";
import type { App } from "./types";

// Minimal helper — only the fields filterAndSort actually reads
function makeApp(partial: {
  uid: string;
  source: App["source"];
  name: string;
  summary?: string | null;
  size_bytes?: number | null;
  install_date?: string | null;
}): App {
  return {
    uid: partial.uid,
    source: partial.source,
    name: partial.name,
    summary: partial.summary ?? null,
    description: null,
    version: null,
    icon_path: null,
    size_bytes: partial.size_bytes ?? null,
    install_date: partial.install_date ?? null,
    publisher: null,
    categories: [],
    exec: null,
    pkg_ref: partial.uid,
    removable: true,
    protected_reason: null,
  };
}

const APPS: App[] = [
  makeApp({ uid: "apt:firefox",   source: "apt",     name: "Firefox",  summary: "Web browser",      size_bytes: 5000, install_date: "2024-03-01" }),
  makeApp({ uid: "apt:gimp",      source: "apt",     name: "GIMP",     summary: "Image editor",      size_bytes: 8000, install_date: "2024-01-15" }),
  makeApp({ uid: "flatpak:vlc",   source: "flatpak", name: "VLC",      summary: "Media player",      size_bytes: 3000, install_date: "2024-02-10" }),
  makeApp({ uid: "snap:discord",  source: "snap",    name: "Discord",  summary: "Chat app",          size_bytes: 9000, install_date: "2024-04-20" }),
  makeApp({ uid: "snap:obs",      source: "snap",    name: "OBS",      summary: "Streaming software", size_bytes: 6000, install_date: "2024-05-01" }),
];

describe("filterAndSort — source filter", () => {
  it('source "all" returns all apps', () => {
    expect(filterAndSort(APPS, "", "all", "name")).toHaveLength(APPS.length);
  });

  it('source "apt" returns only apt apps', () => {
    const result = filterAndSort(APPS, "", "apt", "name");
    expect(result.every((a) => a.source === "apt")).toBe(true);
    expect(result).toHaveLength(2);
  });

  it('source "flatpak" returns only flatpak apps', () => {
    const result = filterAndSort(APPS, "", "flatpak", "name");
    expect(result.every((a) => a.source === "flatpak")).toBe(true);
    expect(result).toHaveLength(1);
  });

  it('source "snap" returns only snap apps', () => {
    const result = filterAndSort(APPS, "", "snap", "name");
    expect(result.every((a) => a.source === "snap")).toBe(true);
    expect(result).toHaveLength(2);
  });
});

describe("filterAndSort — query filter", () => {
  it("filters by name (case-insensitive)", () => {
    const result = filterAndSort(APPS, "firefox", "all", "name");
    expect(result).toHaveLength(1);
    expect(result[0].uid).toBe("apt:firefox");
  });

  it("filters by summary text", () => {
    // "media" matches VLC summary
    const result = filterAndSort(APPS, "media", "all", "name");
    expect(result).toHaveLength(1);
    expect(result[0].uid).toBe("flatpak:vlc");
  });

  it("matches partial name substring", () => {
    // "dis" matches Discord
    const result = filterAndSort(APPS, "dis", "all", "name");
    expect(result).toHaveLength(1);
    expect(result[0].uid).toBe("snap:discord");
  });

  it("combines source and query filter", () => {
    // source=snap, query="stream" matches OBS (summary: "Streaming software")
    const result = filterAndSort(APPS, "stream", "snap", "name");
    expect(result).toHaveLength(1);
    expect(result[0].uid).toBe("snap:obs");
  });

  it("returns empty list when nothing matches", () => {
    expect(filterAndSort(APPS, "zzznomatch", "all", "name")).toHaveLength(0);
  });

  it("empty query returns all (for given source)", () => {
    expect(filterAndSort(APPS, "   ", "all", "name")).toHaveLength(APPS.length);
  });
});

describe("filterAndSort — sort", () => {
  it('sort by name yields alphabetical order', () => {
    const result = filterAndSort(APPS, "", "all", "name");
    const names = result.map((a) => a.name);
    expect(names).toEqual([...names].sort((a, b) => a.localeCompare(b, undefined, { sensitivity: "base" })));
  });

  it('sort by size yields descending size_bytes', () => {
    const result = filterAndSort(APPS, "", "all", "size");
    for (let i = 1; i < result.length; i++) {
      expect(result[i - 1].size_bytes ?? 0).toBeGreaterThanOrEqual(result[i].size_bytes ?? 0);
    }
  });

  it('largest app is first when sorted by size', () => {
    const result = filterAndSort(APPS, "", "all", "size");
    // Discord has 9000 bytes — largest
    expect(result[0].uid).toBe("snap:discord");
  });

  it('sort by recent yields descending install_date', () => {
    const result = filterAndSort(APPS, "", "all", "recent");
    for (let i = 1; i < result.length; i++) {
      const prev = result[i - 1].install_date ?? "";
      const curr = result[i].install_date ?? "";
      expect(prev >= curr).toBe(true);
    }
  });

  it('most recently installed app is first when sorted by recent', () => {
    const result = filterAndSort(APPS, "", "all", "recent");
    // OBS: 2024-05-01 is the latest
    expect(result[0].uid).toBe("snap:obs");
  });
});

describe("filterAndSort — does not mutate input", () => {
  it("original array order is preserved after sort", () => {
    const copy = [...APPS];
    filterAndSort(APPS, "", "all", "size");
    expect(APPS).toEqual(copy);
  });
});
