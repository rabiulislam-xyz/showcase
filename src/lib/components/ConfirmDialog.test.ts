import { describe, it, expect, afterEach, vi } from "vitest";
import { render, screen, cleanup, fireEvent } from "@testing-library/svelte";
import ConfirmDialog from "./ConfirmDialog.svelte";

afterEach(() => {
  cleanup();
});

function baseProps(overrides: Record<string, unknown> = {}) {
  return {
    open: true,
    name: "Firefox",
    sizeBytes: 5_000_000,
    onconfirm: vi.fn(),
    oncancel: vi.fn(),
    ...overrides,
  };
}

describe("ConfirmDialog", () => {
  it("renders the app name in the heading when open", () => {
    render(ConfirmDialog, { props: baseProps() });
    expect(screen.getByRole("heading", { name: /Uninstall Firefox\?/i })).toBeInTheDocument();
  });

  it("renders nothing visible when closed", () => {
    render(ConfirmDialog, { props: baseProps({ open: false }) });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("invokes onconfirm when the Uninstall button is clicked", async () => {
    const onconfirm = vi.fn();
    render(ConfirmDialog, { props: baseProps({ onconfirm }) });

    await fireEvent.click(screen.getByRole("button", { name: /^uninstall$/i }));

    expect(onconfirm).toHaveBeenCalledTimes(1);
  });

  it("invokes oncancel when the Cancel button is clicked", async () => {
    const oncancel = vi.fn();
    render(ConfirmDialog, { props: baseProps({ oncancel }) });

    await fireEvent.click(screen.getByRole("button", { name: /cancel/i }));

    expect(oncancel).toHaveBeenCalledTimes(1);
  });

  it("disables both buttons while busy", () => {
    render(ConfirmDialog, { props: baseProps({ busy: true }) });

    // While busy the confirm button also contains the Spinner ("Loading"),
    // so its accessible name becomes "Loading Uninstall" — match by substring.
    expect(screen.getByRole("button", { name: /cancel/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /uninstall/i })).toBeDisabled();
  });

  it("does not fire callbacks via Escape while busy", async () => {
    const oncancel = vi.fn();
    render(ConfirmDialog, { props: baseProps({ busy: true, oncancel }) });

    await fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });

    expect(oncancel).not.toHaveBeenCalled();
  });

  it("fires oncancel on Escape when not busy", async () => {
    const oncancel = vi.fn();
    render(ConfirmDialog, { props: baseProps({ oncancel }) });

    await fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });

    expect(oncancel).toHaveBeenCalledTimes(1);
  });

  it("shows the apt dependency note only when aptNote is set", () => {
    const { rerender } = render(ConfirmDialog, { props: baseProps({ aptNote: true }) });
    expect(screen.getByText(/dependent packages/i)).toBeInTheDocument();

    cleanup();
    render(ConfirmDialog, { props: baseProps({ aptNote: false }) });
    expect(screen.queryByText(/dependent packages/i)).not.toBeInTheDocument();
    void rerender;
  });
});
