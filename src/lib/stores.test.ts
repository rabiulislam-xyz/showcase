import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { get } from "svelte/store";

// Tauri's core module is unavailable in the Node test environment; stub it out
// so that stores.ts (which transitively imports api.ts → @tauri-apps/api/core)
// can be imported without throwing. invoke is mocked per-test for loadApps.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn((p: string) => p),
}));

import { invoke } from "@tauri-apps/api/core";
import {
  apps,
  selected,
  toasts,
  warnings,
  status,
  errorMsg,
  theme,
  checking,
  removeApp,
  pushToast,
  dismissToast,
  loadApps,
  applyUpdates,
  clearUpdate,
  checkForUpdates,
  updatableApps,
  updatesCount,
  initTheme,
  toggleTheme,
} from "./stores";
import type { App } from "./types";

const mockInvoke = vi.mocked(invoke);

function makeApp(uid: string): App {
  return {
    uid,
    source: "apt",
    name: uid,
    summary: null,
    description: null,
    version: null,
    icon_path: null,
    size_bytes: null,
    install_date: null,
    publisher: null,
    categories: [],
    exec: null,
    desktop_path: null,
    pkg_ref: uid,
    removable: true,
    protected_reason: null,
    update_available: null,
  };
}

beforeEach(() => {
  apps.set([]);
  selected.set(null);
  toasts.set([]);
  warnings.set([]);
  status.set("loading");
  errorMsg.set("");
  checking.set(false);
  mockInvoke.mockReset();
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

// ── removeApp ──────────────────────────────────────────────────────────────────

describe("removeApp", () => {
  it("removes the app and clears selected when it was selected", () => {
    const a = makeApp("apt:a");
    const b = makeApp("apt:b");
    apps.set([a, b]);
    selected.set(a);

    removeApp(a.uid);

    expect(get(apps)).toEqual([b]);
    expect(get(selected)).toBeNull();
  });

  it("removes the app but leaves selected unchanged when a different app is selected", () => {
    const a = makeApp("apt:a");
    const b = makeApp("apt:b");
    apps.set([a, b]);
    selected.set(b);

    removeApp(a.uid);

    expect(get(apps)).toEqual([b]);
    expect(get(selected)).toEqual(b);
  });

  it("leaves apps and selected unchanged when uid is not in the list", () => {
    const a = makeApp("apt:a");
    apps.set([a]);
    selected.set(a);

    removeApp("apt:nonexistent");

    expect(get(apps)).toEqual([a]);
    expect(get(selected)).toEqual(a);
  });
});

// ── pushToast / dismissToast ───────────────────────────────────────────────────

describe("pushToast", () => {
  it("adds one error toast with the correct kind and message", () => {
    pushToast("error", "something went wrong");

    const list = get(toasts);
    expect(list).toHaveLength(1);
    expect(list[0].kind).toBe("error");
    expect(list[0].msg).toBe("something went wrong");
  });

  it("adds one success toast with the correct kind", () => {
    pushToast("success", "done");

    const list = get(toasts);
    expect(list).toHaveLength(1);
    expect(list[0].kind).toBe("success");
  });

  it("auto-removes the toast after 4 seconds", () => {
    pushToast("error", "bye");
    expect(get(toasts)).toHaveLength(1);

    vi.advanceTimersByTime(4000);

    expect(get(toasts)).toHaveLength(0);
  });
});

describe("dismissToast", () => {
  it("removes the toast with the given id", () => {
    pushToast("error", "x");
    const id = get(toasts)[0].id;

    dismissToast(id);

    expect(get(toasts)).toHaveLength(0);
  });

  it("leaves other toasts intact", () => {
    pushToast("error", "first");
    pushToast("success", "second");
    const first = get(toasts)[0];

    dismissToast(first.id);

    const remaining = get(toasts);
    expect(remaining).toHaveLength(1);
    expect(remaining[0].msg).toBe("second");
  });
});

// ── loadApps ─────────────────────────────────────────────────────────────────

describe("loadApps", () => {
  it("sets apps + warnings and status 'ready' on success", async () => {
    const a = makeApp("apt:a");
    mockInvoke.mockResolvedValueOnce({ apps: [a], warnings: ["snap unavailable"] });

    await loadApps();

    expect(mockInvoke).toHaveBeenCalledWith("list_apps");
    expect(get(apps)).toEqual([a]);
    expect(get(warnings)).toEqual(["snap unavailable"]);
    expect(get(status)).toBe("ready");
    expect(get(errorMsg)).toBe("");
  });

  it("sets status 'error' and errorMsg when listApps rejects with an Error", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("backend exploded"));

    await loadApps();

    expect(get(status)).toBe("error");
    expect(get(errorMsg)).toBe("backend exploded");
    expect(get(apps)).toEqual([]);
  });

  it("stringifies a non-Error rejection into errorMsg", async () => {
    mockInvoke.mockRejectedValueOnce("plain string failure");

    await loadApps();

    expect(get(status)).toBe("error");
    expect(get(errorMsg)).toBe("plain string failure");
  });
});

// ── applyUpdates / updatableApps / updatesCount ──────────────────────────────

describe("applyUpdates", () => {
  it("merges update_available into matching apps by uid (immutable)", () => {
    const a = makeApp("apt:a");
    const b = makeApp("apt:b");
    const c = makeApp("apt:c");
    apps.set([a, b, c]);

    applyUpdates([
      ["apt:a", "2.0"],
      ["apt:c", "9.1"],
    ]);

    const list = get(apps);
    expect(list.find((x) => x.uid === "apt:a")?.update_available).toBe("2.0");
    expect(list.find((x) => x.uid === "apt:b")?.update_available).toBeNull();
    expect(list.find((x) => x.uid === "apt:c")?.update_available).toBe("9.1");
    // Original objects are not mutated.
    expect(a.update_available).toBeNull();
    expect(b.update_available).toBeNull();
  });

  it("clears a previously-flagged app when it is absent from a new result", () => {
    const a = { ...makeApp("apt:a"), update_available: "2.0" };
    apps.set([a]);

    applyUpdates([]); // nothing upgradable anymore

    expect(get(apps)[0].update_available).toBeNull();
  });

  it("syncs the open (selected) app's flag with the merged result", () => {
    const a = makeApp("apt:a");
    apps.set([a]);
    selected.set(a);

    applyUpdates([["apt:a", "3.3"]]);

    expect(get(selected)?.update_available).toBe("3.3");
  });

  it("drives updatableApps and updatesCount", () => {
    apps.set([makeApp("apt:a"), makeApp("apt:b"), makeApp("snap:c")]);
    expect(get(updatesCount)).toBe(0);

    applyUpdates([
      ["apt:a", "2.0"],
      ["snap:c", "1.5"],
    ]);

    expect(get(updatableApps).map((x) => x.uid)).toEqual(["apt:a", "snap:c"]);
    expect(get(updatesCount)).toBe(2);
  });
});

describe("clearUpdate", () => {
  it("clears the flag on one app and the open drawer, leaving others", () => {
    const a = { ...makeApp("apt:a"), update_available: "2.0" };
    const b = { ...makeApp("apt:b"), update_available: "3.0" };
    apps.set([a, b]);
    selected.set(a);

    clearUpdate("apt:a");

    const list = get(apps);
    expect(list.find((x) => x.uid === "apt:a")?.update_available).toBeNull();
    expect(list.find((x) => x.uid === "apt:b")?.update_available).toBe("3.0");
    expect(get(selected)?.update_available).toBeNull();
  });
});

describe("checkForUpdates", () => {
  it("toasts an 'N updates available' summary and merges results", async () => {
    apps.set([makeApp("apt:a"), makeApp("apt:b")]);
    mockInvoke.mockResolvedValueOnce([["apt:a", "2.0"]] as [string, string][]);

    await checkForUpdates();

    expect(mockInvoke).toHaveBeenCalledWith("check_updates");
    expect(get(updatesCount)).toBe(1);
    expect(get(checking)).toBe(false);
    const list = get(toasts);
    expect(list).toHaveLength(1);
    expect(list[0].kind).toBe("success");
    expect(list[0].msg).toBe("1 update available");
  });

  it("toasts 'Up to date' when nothing is upgradable", async () => {
    apps.set([makeApp("apt:a")]);
    mockInvoke.mockResolvedValueOnce([] as [string, string][]);

    await checkForUpdates();

    expect(get(toasts)[0].msg).toBe("Up to date");
  });

  it("toasts an error and clears checking when the check rejects", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("check exploded"));

    await checkForUpdates();

    expect(get(checking)).toBe(false);
    const list = get(toasts);
    expect(list).toHaveLength(1);
    expect(list[0].kind).toBe("error");
    expect(list[0].msg).toBe("check exploded");
  });
});

// ── theme: initTheme / toggleTheme ───────────────────────────────────────────

const THEME_KEY = "showcase-theme";

describe("theme store", () => {
  beforeEach(() => {
    localStorage.clear();
    delete document.documentElement.dataset.theme;
    theme.set("light");
  });

  describe("initTheme", () => {
    it("applies a saved 'dark' theme from localStorage", () => {
      localStorage.setItem(THEME_KEY, "dark");

      initTheme();

      expect(get(theme)).toBe("dark");
      expect(document.documentElement.dataset.theme).toBe("dark");
    });

    it("applies a saved 'light' theme from localStorage", () => {
      localStorage.setItem(THEME_KEY, "light");

      initTheme();

      expect(get(theme)).toBe("light");
      expect(document.documentElement.dataset.theme).toBe("light");
    });

    it("falls back to OS preference when nothing is saved", () => {
      // jsdom has no matchMedia by default; stub a dark-preferring matcher.
      vi.stubGlobal(
        "matchMedia",
        vi.fn().mockReturnValue({ matches: true } as MediaQueryList),
      );

      initTheme();

      expect(get(theme)).toBe("dark");
      expect(document.documentElement.dataset.theme).toBe("dark");

      vi.unstubAllGlobals();
    });

    it("defaults to light when no preference and no matchMedia", () => {
      vi.stubGlobal("matchMedia", undefined);

      initTheme();

      expect(get(theme)).toBe("light");
      expect(document.documentElement.dataset.theme).toBe("light");

      vi.unstubAllGlobals();
    });
  });

  describe("toggleTheme", () => {
    it("flips light → dark and persists + applies it", () => {
      theme.set("light");

      toggleTheme();

      expect(get(theme)).toBe("dark");
      expect(document.documentElement.dataset.theme).toBe("dark");
      expect(localStorage.getItem(THEME_KEY)).toBe("dark");
    });

    it("flips dark → light and persists + applies it", () => {
      theme.set("dark");

      toggleTheme();

      expect(get(theme)).toBe("light");
      expect(document.documentElement.dataset.theme).toBe("light");
      expect(localStorage.getItem(THEME_KEY)).toBe("light");
    });
  });
});
