import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, screen, cleanup, fireEvent } from "@testing-library/svelte";
import { get } from "svelte/store";
import { tick } from "svelte";
import Toast from "./Toast.svelte";
import { toasts, pushToast, dismissToast } from "$lib/stores";

beforeEach(() => {
  toasts.set([]);
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("Toast", () => {
  it("renders both a success and an error toast with their text and titles", async () => {
    pushToast("success", "Firefox uninstalled");
    pushToast("error", "Something failed");
    render(Toast);
    await tick();

    expect(screen.getByText("Firefox uninstalled")).toBeInTheDocument();
    expect(screen.getByText("Something failed")).toBeInTheDocument();
    // Kind-specific titles.
    expect(screen.getByText("Done")).toBeInTheDocument();
    expect(screen.getByText("Something went wrong")).toBeInTheDocument();
  });

  it("removes a toast when its dismiss button is clicked, leaving the other", async () => {
    pushToast("error", "first error");
    pushToast("success", "second success");
    render(Toast);
    await tick();

    const dismissButtons = screen.getAllByRole("button", { name: /dismiss notification/i });
    expect(dismissButtons).toHaveLength(2);

    // The error region renders first; dismiss it.
    await fireEvent.click(dismissButtons[0]);
    await tick();

    expect(screen.queryByText("first error")).not.toBeInTheDocument();
    expect(screen.getByText("second success")).toBeInTheDocument();
    expect(get(toasts)).toHaveLength(1);
  });

  it("auto-dismisses a toast after the 4s timeout", async () => {
    pushToast("success", "ephemeral");
    render(Toast);
    await tick();
    expect(screen.getByText("ephemeral")).toBeInTheDocument();

    vi.advanceTimersByTime(4000);
    await tick();

    expect(screen.queryByText("ephemeral")).not.toBeInTheDocument();
  });

  it("reflects an external dismissToast call in the DOM", async () => {
    pushToast("error", "bye");
    render(Toast);
    await tick();
    const id = get(toasts)[0].id;

    dismissToast(id);
    await tick();

    expect(screen.queryByText("bye")).not.toBeInTheDocument();
  });
});
