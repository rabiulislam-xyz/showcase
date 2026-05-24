import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, screen, cleanup, fireEvent, within, waitFor } from "@testing-library/svelte";
import { get } from "svelte/store";
import { tick } from "svelte";
import AppDetail from "./AppDetail.svelte";
import { selected, apps, toasts } from "$lib/stores";
import { getAppDetails, uninstallApp, iconSrc, launchApp, updateApp, checkAppUpdate } from "$lib/api";
import type { App } from "$lib/types";

// Mock only the api boundary; the real stores drive selection/toasts so we can
// assert on the user-visible side effects (toast pushed, app removed).
vi.mock("$lib/api", () => ({
  iconSrc: vi.fn<(app: App) => string | null>(() => null),
  getAppDetails: vi.fn<(uid: string) => Promise<string | null>>(),
  uninstallApp: vi.fn<(uid: string) => Promise<void>>(),
  launchApp: vi.fn<(uid: string) => Promise<void>>(),
  updateApp: vi.fn<(uid: string) => Promise<void>>(),
  checkAppUpdate: vi.fn<(uid: string) => Promise<string | null>>(),
}));

const mockGetDetails = vi.mocked(getAppDetails);
const mockUninstall = vi.mocked(uninstallApp);
const mockIconSrc = vi.mocked(iconSrc);
const mockLaunch = vi.mocked(launchApp);
const mockUpdate = vi.mocked(updateApp);
const mockCheckAppUpdate = vi.mocked(checkAppUpdate);

function makeApp(overrides: Partial<App> = {}): App {
  return {
    uid: "apt:firefox",
    source: "apt",
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
    update_available: null,
    ...overrides,
  };
}

beforeEach(() => {
  selected.set(null);
  apps.set([]);
  toasts.set([]);
  mockGetDetails.mockReset();
  mockUninstall.mockReset();
  mockIconSrc.mockReset();
  mockLaunch.mockReset();
  mockUpdate.mockReset();
  mockCheckAppUpdate.mockReset();
  mockIconSrc.mockReturnValue(null);
  mockGetDetails.mockResolvedValue("Firefox is a free web browser.");
});

afterEach(() => {
  cleanup();
});

/** The footer's Uninstall button (outside the dialog). */
function footerUninstallButton(): HTMLButtonElement {
  const buttons = screen.getAllByRole("button", { name: /uninstall/i });
  const dialog = screen.queryByRole("dialog", { name: /Uninstall .+\?/i });
  const footerBtn = buttons.find((b) => !dialog || !dialog.contains(b));
  if (!footerBtn) throw new Error("footer Uninstall button not found");
  return footerBtn as HTMLButtonElement;
}

describe("AppDetail — rendering", () => {
  it("renders the selected app's name and lazy-loaded description", async () => {
    selected.set(makeApp());
    render(AppDetail);

    expect(screen.getByRole("heading", { name: "Firefox" })).toBeInTheDocument();
    expect(mockGetDetails).toHaveBeenCalledWith("apt:firefox");

    await waitFor(() =>
      expect(screen.getByText("Firefox is a free web browser.")).toBeInTheDocument(),
    );
  });
});

describe("AppDetail — protected app", () => {
  it("disables Uninstall and shows the protected reason as the title", () => {
    selected.set(
      makeApp({ removable: false, protected_reason: "System-critical package" }),
    );
    render(AppDetail);

    const btn = footerUninstallButton();
    expect(btn).toBeDisabled();
    expect(btn).toHaveAttribute("title", "System-critical package");
  });
});

describe("AppDetail — confirm flow", () => {
  it("opens the confirm dialog when Uninstall is clicked", async () => {
    selected.set(makeApp());
    render(AppDetail);

    expect(screen.queryByRole("dialog", { name: /Uninstall Firefox\?/i })).not.toBeInTheDocument();

    await fireEvent.click(footerUninstallButton());

    expect(screen.getByRole("dialog", { name: /Uninstall Firefox\?/i })).toBeInTheDocument();
  });

  it("on confirm success: calls uninstallApp, pushes a success toast, and removes the app", async () => {
    const app = makeApp();
    apps.set([app, makeApp({ uid: "snap:obs", name: "OBS" })]);
    selected.set(app);
    mockUninstall.mockResolvedValueOnce(undefined);
    render(AppDetail);

    await fireEvent.click(footerUninstallButton());
    const dialog = screen.getByRole("dialog", { name: /Uninstall Firefox\?/i });
    await fireEvent.click(within(dialog).getByRole("button", { name: /^uninstall$/i }));
    await tick();

    expect(mockUninstall).toHaveBeenCalledWith("apt:firefox");

    await waitFor(() => {
      const list = get(toasts);
      expect(list).toHaveLength(1);
      expect(list[0].kind).toBe("success");
      expect(list[0].msg).toBe("Firefox uninstalled");
    });

    // App removed from the store; the still-installed app remains.
    expect(get(apps).map((a) => a.uid)).toEqual(["snap:obs"]);
    // removeApp clears selection for the removed (open) app.
    expect(get(selected)).toBeNull();
  });

  it("on PermissionDenied: shows an 'Authentication cancelled' error toast and keeps the app", async () => {
    const app = makeApp();
    apps.set([app]);
    selected.set(app);
    mockUninstall.mockRejectedValueOnce({ kind: "PermissionDenied" });
    render(AppDetail);

    await fireEvent.click(footerUninstallButton());
    const dialog = screen.getByRole("dialog", { name: /Uninstall Firefox\?/i });
    await fireEvent.click(within(dialog).getByRole("button", { name: /^uninstall$/i }));

    await waitFor(() => {
      const list = get(toasts);
      expect(list).toHaveLength(1);
      expect(list[0].kind).toBe("error");
      expect(list[0].msg).toMatch(/Authentication cancelled/i);
    });

    // App stays; selection stays open.
    expect(get(apps).map((a) => a.uid)).toEqual(["apt:firefox"]);
    expect(get(selected)?.uid).toBe("apt:firefox");
  });
});

describe("AppDetail — Open button", () => {
  it("clicking Open calls launchApp and pushes a success toast", async () => {
    const app = makeApp();
    selected.set(app);
    mockLaunch.mockResolvedValueOnce(undefined);
    render(AppDetail);

    await fireEvent.click(screen.getByRole("button", { name: /^open$/i }));

    expect(mockLaunch).toHaveBeenCalledWith("apt:firefox");

    await waitFor(() => {
      const list = get(toasts);
      expect(list).toHaveLength(1);
      expect(list[0].kind).toBe("success");
      expect(list[0].msg).toMatch(/Launching Firefox/i);
    });
  });

  it("a rejected launchApp pushes an error toast", async () => {
    const app = makeApp();
    selected.set(app);
    mockLaunch.mockRejectedValueOnce({ kind: "IoError", message: "No such file" });
    render(AppDetail);

    await fireEvent.click(screen.getByRole("button", { name: /^open$/i }));

    await waitFor(() => {
      const list = get(toasts);
      expect(list).toHaveLength(1);
      expect(list[0].kind).toBe("error");
      expect(list[0].msg).toBe("No such file");
    });
  });
});

describe("AppDetail — Update button", () => {
  it("is hidden when the app has no available update", () => {
    selected.set(makeApp({ update_available: null }));
    render(AppDetail);

    expect(screen.queryByRole("button", { name: /update to/i })).not.toBeInTheDocument();
  });

  it("shows the new version and on success calls updateApp, toasts, and clears the flag", async () => {
    const app = makeApp({ update_available: "125.0" });
    apps.set([app]);
    selected.set(app);
    mockUpdate.mockResolvedValueOnce(undefined);
    render(AppDetail);

    await fireEvent.click(screen.getByRole("button", { name: /update to 125\.0/i }));
    await tick();

    expect(mockUpdate).toHaveBeenCalledWith("apt:firefox");

    await waitFor(() => {
      const list = get(toasts);
      expect(list).toHaveLength(1);
      expect(list[0].kind).toBe("success");
      expect(list[0].msg).toBe("Firefox updated");
    });

    // Flag cleared on the store entry and the open drawer.
    expect(get(apps)[0].update_available).toBeNull();
    expect(get(selected)?.update_available).toBeNull();
  });

  it("on PermissionDenied: shows an 'Authentication cancelled' error toast and keeps the flag", async () => {
    const app = makeApp({ update_available: "125.0" });
    apps.set([app]);
    selected.set(app);
    mockUpdate.mockRejectedValueOnce({ kind: "PermissionDenied" });
    render(AppDetail);

    await fireEvent.click(screen.getByRole("button", { name: /update to 125\.0/i }));

    await waitFor(() => {
      const list = get(toasts);
      expect(list).toHaveLength(1);
      expect(list[0].kind).toBe("error");
      expect(list[0].msg).toBe("Authentication cancelled");
    });

    // Update failed → flag stays so the user can retry.
    expect(get(apps)[0].update_available).toBe("125.0");
  });
});

describe("AppDetail — Check for update button", () => {
  it("is shown only when there is no known update and the source is not appimage", () => {
    selected.set(makeApp({ update_available: null, source: "apt" }));
    render(AppDetail);
    expect(screen.getByRole("button", { name: /check for update/i })).toBeInTheDocument();
  });

  it("is hidden when an update is already known", () => {
    selected.set(makeApp({ update_available: "125.0" }));
    render(AppDetail);
    expect(screen.queryByRole("button", { name: /check for update/i })).not.toBeInTheDocument();
  });

  it("is hidden for AppImage apps", () => {
    selected.set(makeApp({ uid: "appimage:foo", source: "appimage", name: "Foo", update_available: null }));
    render(AppDetail);
    expect(screen.queryByRole("button", { name: /check for update/i })).not.toBeInTheDocument();
  });

  it("when a version is returned: reveals the Update button and toasts the version", async () => {
    const app = makeApp({ update_available: null });
    apps.set([app]);
    selected.set(app);
    mockCheckAppUpdate.mockResolvedValueOnce("2.0");
    render(AppDetail);

    // No Update button before the check.
    expect(screen.queryByRole("button", { name: /update to/i })).not.toBeInTheDocument();

    await fireEvent.click(screen.getByRole("button", { name: /check for update/i }));
    await tick();

    expect(mockCheckAppUpdate).toHaveBeenCalledWith("apt:firefox");

    // applyUpdates flips the flag → the "Update to 2.0" button appears.
    await waitFor(() =>
      expect(screen.getByRole("button", { name: /update to 2\.0/i })).toBeInTheDocument(),
    );

    const list = get(toasts);
    expect(list).toHaveLength(1);
    expect(list[0].kind).toBe("success");
    expect(list[0].msg).toBe("Update available: 2.0");
  });

  it("when null is returned: toasts that the app is up to date and shows no Update button", async () => {
    const app = makeApp({ update_available: null });
    apps.set([app]);
    selected.set(app);
    mockCheckAppUpdate.mockResolvedValueOnce(null);
    render(AppDetail);

    await fireEvent.click(screen.getByRole("button", { name: /check for update/i }));

    await waitFor(() => {
      const list = get(toasts);
      expect(list).toHaveLength(1);
      expect(list[0].kind).toBe("success");
      expect(list[0].msg).toBe("Firefox is up to date");
    });

    expect(screen.queryByRole("button", { name: /update to/i })).not.toBeInTheDocument();
  });

  it("on error: pushes an error toast with the parsed message", async () => {
    const app = makeApp({ update_available: null });
    apps.set([app]);
    selected.set(app);
    mockCheckAppUpdate.mockRejectedValueOnce({ kind: "Backend", message: "apt unavailable" });
    render(AppDetail);

    await fireEvent.click(screen.getByRole("button", { name: /check for update/i }));

    await waitFor(() => {
      const list = get(toasts);
      expect(list).toHaveLength(1);
      expect(list[0].kind).toBe("error");
      expect(list[0].msg).toBe("apt unavailable");
    });
  });
});

// ---------------------------------------------------------------------------
// AppDetail — AppImage confirm wording
// ---------------------------------------------------------------------------

/**
 * Clicks the footer Uninstall button and waits for the ConfirmDialog to appear.
 * The drawer itself is also role="dialog", so we look for the one that has
 * aria-labelledby="cd-title" (which ConfirmDialog sets).
 */
async function openUninstallDialog(): Promise<HTMLElement> {
  const buttons = screen.getAllByRole("button", { name: /uninstall/i });
  // Before click: no cd-title element, so the first (and only) "uninstall" button
  // is the footer button.
  await fireEvent.click(buttons[0]);
  // ConfirmDialog mounts with aria-labelledby="cd-title".
  const dialogs = screen.getAllByRole("dialog");
  const confirmDialog = dialogs.find((d) => d.getAttribute("aria-labelledby") === "cd-title");
  if (!confirmDialog) throw new Error("ConfirmDialog not found after clicking Uninstall");
  return confirmDialog;
}

describe("AppDetail — AppImage delete confirm wording", () => {
  it("shows 'Delete <name>?' as the dialog title for an AppImage app", async () => {
    selected.set(
      makeApp({ uid: "appimage:foo", source: "appimage", name: "Foo AppImage" }),
    );
    render(AppDetail);

    const dialog = await openUninstallDialog();
    expect(dialog).toHaveAccessibleName(/delete foo appimage\?/i);
  });

  it("shows 'permanently deletes the AppImage file' in the dialog body", async () => {
    selected.set(
      makeApp({ uid: "appimage:foo", source: "appimage", name: "Foo", size_bytes: 52_428_800 }),
    );
    render(AppDetail);

    const dialog = await openUninstallDialog();
    expect(dialog).toHaveTextContent(/permanently deletes the appimage file/i);
  });

  it("shows a free-space note in the body when size_bytes is known", async () => {
    selected.set(
      makeApp({ uid: "appimage:foo", source: "appimage", name: "Foo", size_bytes: 52_428_800 }),
    );
    render(AppDetail);

    const dialog = await openUninstallDialog();
    // 52_428_800 bytes = 50 MiB — humanSize will format it as something non-empty.
    // Body must mention size (contains digits from formatted size).
    expect(dialog.textContent).toMatch(/\d/);
    expect(dialog.textContent).toMatch(/permanently deletes the appimage file/i);
  });

  it("confirm button label is 'Delete' for AppImage apps", async () => {
    selected.set(
      makeApp({ uid: "appimage:foo", source: "appimage", name: "Foo" }),
    );
    render(AppDetail);

    const dialog = await openUninstallDialog();
    expect(within(dialog).getByRole("button", { name: /^delete$/i })).toBeInTheDocument();
  });

  it("uses standard 'Uninstall <name>?' wording for apt apps", async () => {
    selected.set(makeApp({ source: "apt", name: "Firefox" }));
    render(AppDetail);

    const dialog = await openUninstallDialog();
    expect(dialog).toHaveAccessibleName(/uninstall firefox\?/i);
    expect(within(dialog).getByRole("button", { name: /^uninstall$/i })).toBeInTheDocument();
  });

  it("uses standard 'Uninstall <name>?' wording for snap apps", async () => {
    selected.set(makeApp({ uid: "snap:vlc", source: "snap", name: "VLC" }));
    render(AppDetail);

    const dialog = await openUninstallDialog();
    expect(dialog).toHaveAccessibleName(/uninstall vlc\?/i);
  });

  it("no Update button for AppImage apps (update_available is null)", () => {
    selected.set(
      makeApp({ uid: "appimage:foo", source: "appimage", name: "Foo", update_available: null }),
    );
    render(AppDetail);

    expect(screen.queryByRole("button", { name: /update to/i })).not.toBeInTheDocument();
  });
});
