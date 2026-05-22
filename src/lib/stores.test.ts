import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";

// Tauri's core module is unavailable in the Node test environment; stub it out
// so that stores.ts (which transitively imports api.ts → @tauri-apps/api/core)
// can be imported without throwing.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn((p: string) => p),
}));

import { apps, selected, toasts, removeApp, pushToast, dismissToast } from "./stores";
import type { App } from "./types";

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
    pkg_ref: uid,
    removable: true,
    protected_reason: null,
  };
}

beforeEach(() => {
  apps.set([]);
  selected.set(null);
  toasts.set([]);
  vi.useFakeTimers();
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
