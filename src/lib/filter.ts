import type { App, Source } from "./types";

export type SortKey = "name" | "size" | "recent";

export function filterAndSort(
  apps: App[],
  query: string,
  source: Source | "all",
  sort: SortKey,
): App[] {
  const q = query.trim().toLowerCase();
  const out = apps.filter(
    (a) =>
      (source === "all" || a.source === source) &&
      (q === "" ||
        a.name.toLowerCase().includes(q) ||
        (a.summary?.toLowerCase().includes(q) ?? false)),
  );
  const by: Record<SortKey, (a: App, b: App) => number> = {
    name: (a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: "base" }),
    size: (a, b) => (b.size_bytes ?? 0) - (a.size_bytes ?? 0),
    recent: (a, b) => (b.install_date ?? "").localeCompare(a.install_date ?? ""),
  };
  return [...out].sort(by[sort]);
}
