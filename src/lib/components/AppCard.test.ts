import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, cleanup, fireEvent } from "@testing-library/svelte";
import { get } from "svelte/store";
import AppCard from "./AppCard.svelte";
import { selected } from "$lib/stores";
import { iconSrc } from "$lib/api";
import type { App } from "$lib/types";

// Mock the api layer so iconSrc is controllable without a Tauri runtime.
vi.mock("$lib/api", () => ({
  iconSrc: vi.fn<(app: App) => string | null>(),
}));

const mockIconSrc = vi.mocked(iconSrc);

function makeApp(overrides: Partial<App> = {}): App {
  return {
    uid: "apt:firefox",
    source: "apt",
    name: "Firefox",
    summary: null,
    description: null,
    version: "120.0",
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
    update_available: null,
    ...overrides,
  };
}

beforeEach(() => {
  selected.set(null);
  mockIconSrc.mockReset();
  mockIconSrc.mockReturnValue(null);
  cleanup();
});

describe("AppCard", () => {
  it("renders the name, source badge label, and version", () => {
    render(AppCard, { props: { app: makeApp() } });

    expect(screen.getByText("Firefox")).toBeInTheDocument();
    expect(screen.getByText("APT")).toBeInTheDocument();
    expect(screen.getByText("120.0")).toBeInTheDocument();
  });

  it("omits the version when the app has none", () => {
    render(AppCard, { props: { app: makeApp({ version: null }) } });

    expect(screen.queryByText("120.0")).not.toBeInTheDocument();
  });

  it("maps a flatpak source to the Flatpak badge", () => {
    render(AppCard, { props: { app: makeApp({ source: "flatpak", uid: "flatpak:vlc", name: "VLC" }) } });

    expect(screen.getByText("Flatpak")).toBeInTheDocument();
  });

  it("sets the selected store when the card is clicked", async () => {
    const app = makeApp();
    render(AppCard, { props: { app } });

    await fireEvent.click(screen.getByRole("button"));

    expect(get(selected)).toEqual(app);
  });

  it("falls back to the letter tile when icon_path is null", () => {
    mockIconSrc.mockReturnValue(null);
    const { container } = render(AppCard, {
      props: { app: makeApp({ name: "Firefox", icon_path: null }) },
    });

    // No <img>; the letter tile shows the uppercased initial.
    // alt="" makes the icon presentational, so query the element directly.
    expect(container.querySelector("img")).toBeNull();
    expect(screen.getByText("F")).toBeInTheDocument();
  });

  it("renders an <img> when iconSrc returns a source", () => {
    mockIconSrc.mockReturnValue("asset://localhost/firefox.png");
    const { container } = render(AppCard, {
      props: { app: makeApp({ icon_path: "/x/firefox.png" }) },
    });

    const img = container.querySelector("img");
    expect(img).toHaveAttribute("src", "asset://localhost/firefox.png");
  });

  it("reflects the selected state via aria-pressed", () => {
    const app = makeApp();
    selected.set(app);
    render(AppCard, { props: { app } });

    expect(screen.getByRole("button")).toHaveAttribute("aria-pressed", "true");
  });

  it("renders the update-available badge when the app is flagged", () => {
    render(AppCard, { props: { app: makeApp({ update_available: "2.0" }) } });

    expect(screen.getByLabelText("Update available")).toBeInTheDocument();
  });

  it("omits the update-available badge when not flagged", () => {
    render(AppCard, { props: { app: makeApp({ update_available: null }) } });

    expect(screen.queryByLabelText("Update available")).not.toBeInTheDocument();
  });
});
