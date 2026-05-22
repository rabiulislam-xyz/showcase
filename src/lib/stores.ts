import { writable, derived } from "svelte/store";
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
