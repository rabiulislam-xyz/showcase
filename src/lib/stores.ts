import { writable, derived, get } from "svelte/store";
import type { App } from "./types";
import { listApps } from "./api";
import { filterAndSort } from "./filter";
import type { SortKey } from "./filter";

// --- Writable stores ---

export const query = writable<string>("");
export const sourceFilter = writable<"all" | "apt" | "flatpak" | "snap">("all");
export const sortKey = writable<SortKey>("name");
export const apps = writable<App[]>([]);
export const warnings = writable<string[]>([]);
export const selected = writable<App | null>(null);
export const status = writable<"loading" | "ready" | "error">("loading");
export const errorMsg = writable<string>("");

// --- Derived store ---

export const visibleApps = derived(
  [apps, query, sourceFilter, sortKey],
  ([$apps, $query, $sourceFilter, $sortKey]) =>
    filterAndSort($apps, $query, $sourceFilter, $sortKey),
);

// --- Async action ---

export async function loadApps(): Promise<void> {
  status.set("loading");
  errorMsg.set("");
  try {
    const result = await listApps();
    apps.set(result.apps);
    warnings.set(result.warnings);
    status.set("ready");
  } catch (e) {
    errorMsg.set(e instanceof Error ? e.message : String(e));
    status.set("error");
  }
}

/** Remove an app from the list; clear selection if it was the removed app. */
export function removeApp(uid: string): void {
  apps.update((list) => list.filter((a) => a.uid !== uid));
  // Clear selected if the removed app was open in the detail drawer.
  if (get(selected)?.uid === uid) {
    selected.set(null);
  }
}

// --- Toast store ---

export interface Toast {
  id: number;
  kind: "success" | "error";
  msg: string;
}

let _nextToastId = 0;

export const toasts = writable<Toast[]>([]);

export function pushToast(kind: "success" | "error", msg: string): void {
  const id = _nextToastId++;
  toasts.update((list) => [...list, { id, kind, msg }]);
  setTimeout(() => dismissToast(id), 4000);
}

export function dismissToast(id: number): void {
  toasts.update((list) => list.filter((t) => t.id !== id));
}
