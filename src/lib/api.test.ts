import { describe, it, expect, beforeEach, vi } from "vitest";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { listApps, getAppDetails, uninstallApp, iconSrc } from "./api";
import type { App, AppList } from "./types";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn((p: string) => `asset://localhost/${p}`),
}));

const mockInvoke = vi.mocked(invoke);
const mockConvert = vi.mocked(convertFileSrc);

function makeApp(overrides: Partial<App> = {}): App {
  return {
    uid: "apt:firefox",
    source: "apt",
    name: "Firefox",
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
    pkg_ref: "firefox",
    removable: true,
    protected_reason: null,
    ...overrides,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("listApps", () => {
  it("invokes the list_apps command and returns its result", async () => {
    const payload: AppList = { apps: [makeApp()], warnings: ["heads up"] };
    mockInvoke.mockResolvedValueOnce(payload);

    const result = await listApps();

    expect(mockInvoke).toHaveBeenCalledTimes(1);
    expect(mockInvoke).toHaveBeenCalledWith("list_apps");
    expect(result).toBe(payload);
  });
});

describe("getAppDetails", () => {
  it("invokes get_app_details with the uid argument", async () => {
    mockInvoke.mockResolvedValueOnce("A long description.");

    const result = await getAppDetails("snap:obs");

    expect(mockInvoke).toHaveBeenCalledWith("get_app_details", { uid: "snap:obs" });
    expect(result).toBe("A long description.");
  });

  it("propagates a null description", async () => {
    mockInvoke.mockResolvedValueOnce(null);
    await expect(getAppDetails("apt:x")).resolves.toBeNull();
  });
});

describe("uninstallApp", () => {
  it("invokes uninstall_app with the uid argument", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    await uninstallApp("flatpak:vlc");

    expect(mockInvoke).toHaveBeenCalledWith("uninstall_app", { uid: "flatpak:vlc" });
  });
});

describe("iconSrc", () => {
  it("returns null when icon_path is null and does not call convertFileSrc", () => {
    const app = makeApp({ icon_path: null });

    expect(iconSrc(app)).toBeNull();
    expect(mockConvert).not.toHaveBeenCalled();
  });

  it("converts the icon_path via convertFileSrc when present", () => {
    const app = makeApp({ icon_path: "/usr/share/icons/firefox.png" });

    const result = iconSrc(app);

    expect(mockConvert).toHaveBeenCalledWith("/usr/share/icons/firefox.png");
    expect(result).toBe("asset://localhost//usr/share/icons/firefox.png");
  });
});
