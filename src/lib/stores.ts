import { writable, derived, get } from "svelte/store";
import type { App, Source } from "./types";
import { listApps, checkUpdates } from "./api";
import { filterAndSort } from "./filter";
import type { SortKey, SortDir } from "./filter";
import { availableCategories } from "./categories";

// --- Writable stores ---

export const query = writable<string>("");
export const sourceFilter = writable<"all" | Source>("all");
export const sortKey = writable<SortKey>("name");
export const sortDir = writable<SortDir>("asc");
export const categoryFilter = writable<string>("all");
export const apps = writable<App[]>([]);
export const warnings = writable<string[]>([]);
export const selected = writable<App | null>(null);
export const status = writable<"loading" | "ready" | "error">("loading");
export const errorMsg = writable<string>("");
/** True while an update check is in flight (drives the header spinner). */
export const checking = writable<boolean>(false);

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

/** Apps the most recent check flagged as having an available update. */
export const updatableApps = derived(apps, ($apps) =>
  $apps.filter((a) => a.update_available !== null),
);

export const updatesCount = derived(updatableApps, ($updatable) => $updatable.length);

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

/**
 * Merge check results into the app list by uid (immutable).
 * Each pair is [uid, available_version]; apps not in the result keep their
 * existing flag, so re-checking only flips entries that actually changed.
 */
export function applyUpdates(pairs: [string, string][]): void {
  const versions = new Map(pairs);
  apps.update((list) =>
    list.map((a) => {
      const version = versions.get(a.uid) ?? null;
      return version === a.update_available ? a : { ...a, update_available: version };
    }),
  );
  // Keep the open drawer in sync with the merged flag.
  const sel = get(selected);
  if (sel) {
    const version = versions.get(sel.uid) ?? null;
    if (version !== sel.update_available) {
      selected.set({ ...sel, update_available: version });
    }
  }
}

/** Clear the update flag on a single app (after it has been updated). */
export function clearUpdate(uid: string): void {
  apps.update((list) =>
    list.map((a) => (a.uid === uid ? { ...a, update_available: null } : a)),
  );
  const sel = get(selected);
  if (sel?.uid === uid && sel.update_available !== null) {
    selected.set({ ...sel, update_available: null });
  }
}

/**
 * Check every source for updates, merge the results, and toast a summary.
 * Per-source failures surface from the backend as a rejected promise.
 */
export async function checkForUpdates(): Promise<void> {
  checking.set(true);
  try {
    const pairs = await checkUpdates();
    applyUpdates(pairs);
    const n = pairs.length;
    pushToast("success", n > 0 ? `${n} update${n === 1 ? "" : "s"} available` : "Up to date");
  } catch (e) {
    pushToast("error", e instanceof Error ? e.message : String(e));
  } finally {
    checking.set(false);
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
