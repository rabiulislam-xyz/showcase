import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, screen, cleanup, fireEvent, within, waitFor } from "@testing-library/svelte";
import { get } from "svelte/store";
import { tick } from "svelte";
import Header from "./Header.svelte";
import { apps, toasts, updatableApps } from "$lib/stores";
import { updateAll } from "$lib/api";
import type { App, Source } from "$lib/types";

vi.mock("$lib/api", () => ({
  updateAll: vi.fn<(uids: string[]) => Promise<{ updated: string[]; errors: string[] }>>(),
  // Other api exports referenced transitively by stores are not needed here,
  // but we stub them to avoid Tauri invoke errors in jsdom.
  listApps: vi.fn(),
  checkUpdates: vi.fn(),
  updateApp: vi.fn(),
  uninstallApp: vi.fn(),
  launchApp: vi.fn(),
  getAppDetails: vi.fn(),
  iconSrc: vi.fn(() => null),
}));

const mockUpdateAll = vi.mocked(updateAll);

function makeApp(overrides: Partial<App> = {}): App {
  return {
    uid: "apt:firefox",
    source: "apt" as Source,
    name: "Firefox",
    summary: null,
    description: null,
    version: "120.0",
    icon_path: null,
    size_bytes: 5_000_000,
    install_date: "2024-03-01",
    publisher: "Mozilla",
    categories: [],
    exec: null,
    desktop_path: null,
    pkg_ref: "firefox",
    removable: true,
    protected_reason: null,
    update_available: "121.0",
    ...overrides,
  };
}

beforeEach(() => {
  apps.set([]);
  toasts.set([]);
  mockUpdateAll.mockReset();
});

afterEach(() => {
  cleanup();
});

describe("Header — Update all dialog", () => {
  it("renders the Update all button when updatableApps is non-empty", () => {
    apps.set([makeApp()]);
    render(Header);

    expect(screen.getByRole("button", { name: /update all/i })).toBeInTheDocument();
  });

  it("does not render the Update all button when no updates are available", () => {
    apps.set([makeApp({ update_available: null })]);
    render(Header);

    expect(screen.queryByRole("button", { name: /update all/i })).not.toBeInTheDocument();
  });

  it("clicking Update all opens the ConfirmDialog", async () => {
    apps.set([makeApp()]);
    render(Header);

    // ConfirmDialog is closed initially.
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

    await fireEvent.click(screen.getByRole("button", { name: /update all/i }));

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    // The dialog title should mention the count.
    expect(screen.getByRole("dialog")).toHaveAccessibleName(/update all \(1\)\?/i);
  });

  it("cancel closes the dialog without calling updateAll", async () => {
    apps.set([makeApp()]);
    render(Header);

    await fireEvent.click(screen.getByRole("button", { name: /update all/i }));
    const dialog = screen.getByRole("dialog");
    await fireEvent.click(within(dialog).getByRole("button", { name: /cancel/i }));

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(mockUpdateAll).not.toHaveBeenCalled();
  });

  it("confirming calls updateAll and clears the update badge for each updated uid", async () => {
    const app = makeApp();
    apps.set([app]);
    mockUpdateAll.mockResolvedValueOnce({ updated: [app.uid], errors: [] });
    render(Header);

    await fireEvent.click(screen.getByRole("button", { name: /update all/i }));
    const dialog = screen.getByRole("dialog");
    await fireEvent.click(within(dialog).getByRole("button", { name: /update all/i }));
    await tick();

    expect(mockUpdateAll).toHaveBeenCalledWith([app.uid]);

    await waitFor(() => {
      // Badge cleared: the app's update_available should be null in the store.
      expect(get(apps)[0].update_available).toBeNull();
    });

    // Success toast pushed.
    const list = get(toasts);
    expect(list.length).toBeGreaterThan(0);
    expect(list.some((t) => t.kind === "success" && /updated/i.test(t.msg))).toBe(true);
  });

  it("does NOT call loadApps after updating — uses per-uid clearUpdate instead", async () => {
    const app = makeApp();
    apps.set([app]);
    mockUpdateAll.mockResolvedValueOnce({ updated: [app.uid], errors: [] });
    render(Header);

    await fireEvent.click(screen.getByRole("button", { name: /update all/i }));
    const dialog = screen.getByRole("dialog");
    await fireEvent.click(within(dialog).getByRole("button", { name: /update all/i }));
    await tick();

    // Verify the store was mutated surgically: only update_available is cleared,
    // not the entire app list (which loadApps would replace).
    await waitFor(() => {
      const stored = get(apps);
      expect(stored).toHaveLength(1);
      expect(stored[0].uid).toBe(app.uid);
      expect(stored[0].update_available).toBeNull();
    });
  });

  it("surfaces the first error message in the error toast", async () => {
    const app = makeApp();
    apps.set([app]);
    mockUpdateAll.mockResolvedValueOnce({
      updated: [],
      errors: ["apt-get failed: permission denied"],
    });
    render(Header);

    await fireEvent.click(screen.getByRole("button", { name: /update all/i }));
    const dialog = screen.getByRole("dialog");
    await fireEvent.click(within(dialog).getByRole("button", { name: /update all/i }));
    await tick();

    await waitFor(() => {
      const list = get(toasts);
      const errToast = list.find((t) => t.kind === "error");
      expect(errToast).toBeDefined();
      expect(errToast?.msg).toMatch(/apt-get failed/i);
    });
  });

  it("shows both a success and an error toast when there are partial failures", async () => {
    const appA = makeApp({ uid: "apt:firefox", name: "Firefox" });
    const appB = makeApp({ uid: "apt:vlc", name: "VLC" });
    apps.set([appA, appB]);
    mockUpdateAll.mockResolvedValueOnce({
      updated: [appA.uid],
      errors: ["vlc failed"],
    });
    render(Header);

    await fireEvent.click(screen.getByRole("button", { name: /update all/i }));
    const dialog = screen.getByRole("dialog");
    await fireEvent.click(within(dialog).getByRole("button", { name: /update all/i }));
    await tick();

    await waitFor(() => {
      const list = get(toasts);
      expect(list.some((t) => t.kind === "success")).toBe(true);
      expect(list.some((t) => t.kind === "error" && t.msg.includes("vlc failed"))).toBe(true);
    });
  });

  it("updatableApps derived store reflects the remaining unflagged apps", async () => {
    const appA = makeApp({ uid: "apt:firefox", name: "Firefox" });
    const appB = makeApp({ uid: "apt:vlc", name: "VLC", update_available: "3.0" });
    apps.set([appA, appB]);
    mockUpdateAll.mockResolvedValueOnce({ updated: [appA.uid], errors: [] });
    render(Header);

    await fireEvent.click(screen.getByRole("button", { name: /update all/i }));
    const dialog = screen.getByRole("dialog");
    await fireEvent.click(within(dialog).getByRole("button", { name: /update all/i }));
    await tick();

    await waitFor(() => {
      // Only appB remains updatable after firefox is cleared.
      const updatable = get(updatableApps);
      expect(updatable.map((a) => a.uid)).toEqual([appB.uid]);
    });
  });
});

describe("Header — AppImage source chip", () => {
  it("renders an AppImage filter chip in the segmented control", () => {
    apps.set([]);
    render(Header);

    // The segmented control must include an AppImage button.
    const group = screen.getByRole("group", { name: /filter by source/i });
    expect(within(group).getByRole("button", { name: /appimage/i })).toBeInTheDocument();
  });

  it("shows count 0 when no AppImage apps are present", () => {
    apps.set([makeApp({ source: "apt" })]);
    render(Header);

    const group = screen.getByRole("group", { name: /filter by source/i });
    const chip = within(group).getByRole("button", { name: /appimage/i });
    // The count span inside the chip should read "0".
    expect(chip).toHaveTextContent("0");
  });

  it("reflects the correct AppImage app count", () => {
    apps.set([
      makeApp({ uid: "appimage:foo", source: "appimage", name: "Foo" }),
      makeApp({ uid: "appimage:bar", source: "appimage", name: "Bar" }),
      makeApp({ source: "apt" }),
    ]);
    render(Header);

    const group = screen.getByRole("group", { name: /filter by source/i });
    const chip = within(group).getByRole("button", { name: /appimage/i });
    // Chip text is "AppImage 2" (label + count).
    expect(chip).toHaveTextContent("2");
  });
});
