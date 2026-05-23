import type { App } from "./types";

/**
 * Freedesktop "main" category groups, in priority order. The first rule whose
 * trigger tag is present in an app's raw .desktop categories wins, so more
 * specific groups (Game, Development) take precedence over broad ones (Utility).
 */
const PRIORITY: { group: string; triggers: string[] }[] = [
  { group: "Games", triggers: ["Game"] },
  { group: "Development", triggers: ["Development"] },
  { group: "Graphics", triggers: ["Graphics"] },
  { group: "Audio & Video", triggers: ["AudioVideo", "Audio", "Video"] },
  { group: "Internet", triggers: ["Network"] },
  { group: "Office", triggers: ["Office"] },
  { group: "Science", triggers: ["Science"] },
  { group: "Education", triggers: ["Education"] },
  { group: "Settings", triggers: ["Settings"] },
  { group: "System", triggers: ["System"] },
  { group: "Utilities", triggers: ["Utility"] },
];

export const OTHER_CATEGORY = "Other";

/** Map an app's raw .desktop category tags to a single main group. */
export function mainCategory(categories: string[]): string {
  const tags = new Set(categories);
  for (const { group, triggers } of PRIORITY) {
    if (triggers.some((t) => tags.has(t))) return group;
  }
  return OTHER_CATEGORY;
}

/** Distinct main groups present across apps, sorted alphabetically with "Other" last. */
export function availableCategories(apps: App[]): string[] {
  const present = new Set(apps.map((a) => mainCategory(a.categories)));
  const hasOther = present.delete(OTHER_CATEGORY);
  const sorted = [...present].sort((a, b) => a.localeCompare(b));
  if (hasOther) sorted.push(OTHER_CATEGORY);
  return sorted;
}
