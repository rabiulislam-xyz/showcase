import type { Source } from "./types";

/** Brand-aligned palette for letter-tile fallbacks (design colors). */
const TILE_PALETTE = [
  "#D97757", // accent
  "#B07A2B", // apt
  "#4E837F", // flatpak
  "#8E5A85", // snap
  "#6A9BCC",
  "#788C5D",
] as const;

/** Pick a stable tile color from a name via a small deterministic hash. */
export function tileColor(name: string): string {
  let hash = 0;
  for (let i = 0; i < name.length; i += 1) {
    hash = (hash * 31 + name.charCodeAt(i)) | 0;
  }
  const index = Math.abs(hash) % TILE_PALETTE.length;
  return TILE_PALETTE[index];
}

/** First letter of a name for the tile, uppercased, with a safe fallback. */
export function tileInitial(name: string): string {
  return name.trim().charAt(0).toUpperCase() || "?";
}

const SOURCE_LABELS: Record<Source, string> = {
  apt: "APT",
  flatpak: "Flatpak",
  snap: "Snap",
  appimage: "AppImage",
};

/** Human display label for a source tag. */
export function sourceLabel(source: Source): string {
  return SOURCE_LABELS[source];
}
