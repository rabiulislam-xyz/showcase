import type { App, Source } from "./types";
import { mainCategory } from "./categories";

export type SortKey = "name" | "size" | "recent";
export type SortDir = "asc" | "desc";

// Ascending comparators: name A→Z, size smallest→largest, recent oldest→newest.
// Nulls sort as the smallest value (empty string / 0) so they cluster at the
// ascending end regardless of direction.
const ASC: Record<SortKey, (a: App, b: App) => number> = {
  name: (a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: "base" }),
  size: (a, b) => (a.size_bytes ?? 0) - (b.size_bytes ?? 0),
  recent: (a, b) => (a.install_date ?? "").localeCompare(b.install_date ?? ""),
};

export function filterAndSort(
  apps: App[],
  query: string,
  source: Source | "all",
  sort: SortKey,
  dir: SortDir,
  category: string,
): App[] {
  const q = query.trim().toLowerCase();
  const out = apps.filter(
    (a) =>
      (source === "all" || a.source === source) &&
      (category === "all" || mainCategory(a.categories) === category) &&
      (q === "" ||
        a.name.toLowerCase().includes(q) ||
        (a.summary?.toLowerCase().includes(q) ?? false)),
  );
  const cmp = ASC[sort];
  const directed = dir === "desc" ? (a: App, b: App) => -cmp(a, b) : cmp;
  return [...out].sort(directed);
}
