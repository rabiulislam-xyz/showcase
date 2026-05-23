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
  categories?: string[];
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
    categories: partial.categories ?? [],
    exec: null,
    desktop_path: null,
    pkg_ref: partial.uid,
    removable: true,
    protected_reason: null,
    update_available: null,
  };
}

const APPS: App[] = [
  makeApp({ uid: "apt:firefox",   source: "apt",     name: "Firefox",  summary: "Web browser",       size_bytes: 5000, install_date: "2024-03-01", categories: ["Network"] }),
  makeApp({ uid: "apt:gimp",      source: "apt",     name: "GIMP",     summary: "Image editor",       size_bytes: 8000, install_date: "2024-01-15", categories: ["Graphics"] }),
  makeApp({ uid: "flatpak:vlc",   source: "flatpak", name: "VLC",      summary: "Media player",       size_bytes: 3000, install_date: "2024-02-10", categories: ["AudioVideo"] }),
  makeApp({ uid: "snap:discord",  source: "snap",    name: "Discord",  summary: "Chat app",           size_bytes: 9000, install_date: "2024-04-20", categories: ["Network"] }),
  makeApp({ uid: "snap:obs",      source: "snap",    name: "OBS",      summary: "Streaming software", size_bytes: 6000, install_date: "2024-05-01", categories: ["AudioVideo"] }),
];

describe("filterAndSort — source filter", () => {
  it('source "all" returns all apps', () => {
    expect(filterAndSort(APPS, "", "all", "name", "asc", "all")).toHaveLength(APPS.length);
  });

  it('source "apt" returns only apt apps', () => {
    const result = filterAndSort(APPS, "", "apt", "name", "asc", "all");
    expect(result.every((a) => a.source === "apt")).toBe(true);
    expect(result).toHaveLength(2);
  });

  it('source "flatpak" returns only flatpak apps', () => {
    const result = filterAndSort(APPS, "", "flatpak", "name", "asc", "all");
    expect(result.every((a) => a.source === "flatpak")).toBe(true);
    expect(result).toHaveLength(1);
  });

  it('source "snap" returns only snap apps', () => {
    const result = filterAndSort(APPS, "", "snap", "name", "asc", "all");
    expect(result.every((a) => a.source === "snap")).toBe(true);
    expect(result).toHaveLength(2);
  });
});

describe("filterAndSort — query filter", () => {
  it("filters by name (case-insensitive)", () => {
    const result = filterAndSort(APPS, "firefox", "all", "name", "asc", "all");
    expect(result).toHaveLength(1);
    expect(result[0].uid).toBe("apt:firefox");
  });

  it("filters by summary text", () => {
    // "media" matches VLC summary
    const result = filterAndSort(APPS, "media", "all", "name", "asc", "all");
    expect(result).toHaveLength(1);
    expect(result[0].uid).toBe("flatpak:vlc");
  });

  it("matches partial name substring", () => {
    // "dis" matches Discord
    const result = filterAndSort(APPS, "dis", "all", "name", "asc", "all");
    expect(result).toHaveLength(1);
    expect(result[0].uid).toBe("snap:discord");
  });

  it("combines source and query filter", () => {
    // source=snap, query="stream" matches OBS (summary: "Streaming software")
    const result = filterAndSort(APPS, "stream", "snap", "name", "asc", "all");
    expect(result).toHaveLength(1);
    expect(result[0].uid).toBe("snap:obs");
  });

  it("returns empty list when nothing matches", () => {
    expect(filterAndSort(APPS, "zzznomatch", "all", "name", "asc", "all")).toHaveLength(0);
  });

  it("empty query returns all (for given source)", () => {
    expect(filterAndSort(APPS, "   ", "all", "name", "asc", "all")).toHaveLength(APPS.length);
  });
});

describe("filterAndSort — sort", () => {
  it('sort by name asc yields A→Z order', () => {
    const result = filterAndSort(APPS, "", "all", "name", "asc", "all");
    const names = result.map((a) => a.name);
    expect(names).toEqual([...names].sort((a, b) => a.localeCompare(b, undefined, { sensitivity: "base" })));
  });

  it('sort by name desc reverses A→Z order', () => {
    const asc = filterAndSort(APPS, "", "all", "name", "asc", "all").map((a) => a.name);
    const desc = filterAndSort(APPS, "", "all", "name", "desc", "all").map((a) => a.name);
    expect(desc).toEqual([...asc].reverse());
  });

  it('sort by size asc yields smallest→largest', () => {
    const result = filterAndSort(APPS, "", "all", "size", "asc", "all");
    for (let i = 1; i < result.length; i++) {
      expect(result[i - 1].size_bytes ?? 0).toBeLessThanOrEqual(result[i].size_bytes ?? 0);
    }
    // VLC (3000) is the smallest
    expect(result[0].uid).toBe("flatpak:vlc");
  });

  it('sort by size desc yields largest→smallest', () => {
    const result = filterAndSort(APPS, "", "all", "size", "desc", "all");
    for (let i = 1; i < result.length; i++) {
      expect(result[i - 1].size_bytes ?? 0).toBeGreaterThanOrEqual(result[i].size_bytes ?? 0);
    }
    // Discord has 9000 bytes — largest
    expect(result[0].uid).toBe("snap:discord");
  });

  it('sort by recent asc yields oldest→newest', () => {
    const result = filterAndSort(APPS, "", "all", "recent", "asc", "all");
    for (let i = 1; i < result.length; i++) {
      const prev = result[i - 1].install_date ?? "";
      const curr = result[i].install_date ?? "";
      expect(prev <= curr).toBe(true);
    }
    // GIMP: 2024-01-15 is the oldest
    expect(result[0].uid).toBe("apt:gimp");
  });

  it('sort by recent desc yields newest→oldest', () => {
    const result = filterAndSort(APPS, "", "all", "recent", "desc", "all");
    for (let i = 1; i < result.length; i++) {
      const prev = result[i - 1].install_date ?? "";
      const curr = result[i].install_date ?? "";
      expect(prev >= curr).toBe(true);
    }
    // OBS: 2024-05-01 is the latest
    expect(result[0].uid).toBe("snap:obs");
  });
});

describe("filterAndSort — category filter", () => {
  it('"all" category passes every app through', () => {
    const result = filterAndSort(APPS, "", "all", "name", "asc", "all");
    expect(result).toHaveLength(APPS.length);
  });

  it('filters to "Internet" (Network tag) — Firefox + Discord', () => {
    const result = filterAndSort(APPS, "", "all", "name", "asc", "Internet");
    expect(result).toHaveLength(2);
    expect(result.map((a) => a.uid).sort()).toEqual(["apt:firefox", "snap:discord"].sort());
  });

  it('filters to "Audio & Video" (AudioVideo tag) — VLC + OBS', () => {
    const result = filterAndSort(APPS, "", "all", "name", "asc", "Audio & Video");
    expect(result).toHaveLength(2);
    expect(result.map((a) => a.uid).sort()).toEqual(["flatpak:vlc", "snap:obs"].sort());
  });

  it('filters to "Graphics" — GIMP only', () => {
    const result = filterAndSort(APPS, "", "all", "name", "asc", "Graphics");
    expect(result).toHaveLength(1);
    expect(result[0].uid).toBe("apt:gimp");
  });

  it('a category with no matching apps returns empty', () => {
    expect(filterAndSort(APPS, "", "all", "name", "asc", "Games")).toHaveLength(0);
  });
});

describe("filterAndSort — does not mutate input", () => {
  it("original array order is preserved after sort", () => {
    const copy = [...APPS];
    filterAndSort(APPS, "", "all", "size", "desc", "all");
    expect(APPS).toEqual(copy);
  });
});
