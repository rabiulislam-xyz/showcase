import { writable, derived, get } from "svelte/store";
import type { App } from "./types";
import { listApps } from "./api";
import { filterAndSort } from "./filter";
import type { SortKey, SortDir } from "./filter";
import { availableCategories } from "./categories";

// --- Writable stores ---

export const query = writable<string>("");
export const sourceFilter = writable<"all" | "apt" | "flatpak" | "snap">("all");
export const sortKey = writable<SortKey>("name");
export const sortDir = writable<SortDir>("asc");
export const categoryFilter = writable<string>("all");
export const apps = writable<App[]>([]);
export const warnings = writable<string[]>([]);
export const selected = writable<App | null>(null);
export const status = writable<"loading" | "ready" | "error">("loading");
export const errorMsg = writable<string>("");

// --- Sort actions ---

/** The natural starting direction for a key: alphabetical asc, others newest/largest first. */
export function defaultDir(key: SortKey): SortDir {
  return key === "name" ? "asc" : "desc";
}

/** Select a sort key and reset the direction to that key's natural default. */
export function setSort(key: SortKey): void {
  sortKey.set(key);
  sortDir.set(defaultDir(key));
}

export function toggleSortDir(): void {
  sortDir.update((d) => (d === "asc" ? "desc" : "asc"));
}

// --- Derived stores ---

/** Distinct main category groups present in the current app list (Other last). */
export const availableCats = derived(apps, ($apps) => availableCategories($apps));

export const visibleApps = derived(
  [apps, query, sourceFilter, sortKey, sortDir, categoryFilter],
  ([$apps, $query, $sourceFilter, $sortKey, $sortDir, $categoryFilter]) =>
    filterAndSort($apps, $query, $sourceFilter, $sortKey, $sortDir, $categoryFilter),
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

// --- Theme store ---

export type Theme = "light" | "dark";

const THEME_KEY = "showcase-theme";

export const theme = writable<Theme>("light");

/** Apply a theme to <html> via the data-theme attribute (SSR-safe). */
function applyTheme(t: Theme): void {
  if (typeof document === "undefined") return;
  document.documentElement.dataset.theme = t;
}

/** Resolve the initial theme from storage, falling back to OS preference. */
export function initTheme(): void {
  let initial: Theme = "light";
  try {
    const saved = localStorage.getItem(THEME_KEY);
    if (saved === "light" || saved === "dark") {
      initial = saved;
    } else if (typeof matchMedia === "function" && matchMedia("(prefers-color-scheme: dark)").matches) {
      initial = "dark";
    }
  } catch {
    // localStorage/matchMedia unavailable — keep the light default.
  }
  theme.set(initial);
  applyTheme(initial);
}

export function toggleTheme(): void {
  theme.update((current) => {
    const next: Theme = current === "dark" ? "light" : "dark";
    applyTheme(next);
    try {
      localStorage.setItem(THEME_KEY, next);
    } catch {
      // Persistence is best-effort; ignore storage failures.
    }
    return next;
  });
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
