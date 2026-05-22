const PLACEHOLDER = "—";

const SIZE_UNITS = ["B", "KB", "MB", "GB", "TB", "PB"] as const;

/** Human-readable byte size (1024-based), e.g. 97086620 -> "92.6 MB". */
export function humanSize(bytes: number | null): string {
  if (bytes === null || !Number.isFinite(bytes) || bytes < 0) return PLACEHOLDER;
  if (bytes < 1024) return `${bytes} B`;

  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < SIZE_UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  // One decimal place, trimming a trailing ".0".
  const rounded = Math.round(value * 10) / 10;
  const text = Number.isInteger(rounded) ? String(rounded) : rounded.toFixed(1);
  return `${text} ${SIZE_UNITS[unit]}`;
}

/** Locale date from an RFC3339 string, e.g. "May 23, 2026". */
export function humanDate(iso: string | null): string {
  if (!iso) return PLACEHOLDER;
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return PLACEHOLDER;
  return date.toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}
