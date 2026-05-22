<script lang="ts">
  import type { Source } from "$lib/types";

  let { source }: { source: Source } = $props();

  const colorVar: Record<Source, string> = {
    apt: "var(--source-apt)",
    flatpak: "var(--source-flatpak)",
    snap: "var(--source-snap)",
  };
</script>

<span class="badge" style="--badge-color: {colorVar[source]}">{source}</span>

<style>
  .badge {
    display: inline-block;
    padding: 1px 8px;
    border-radius: var(--radius-pill);
    background: var(--badge-color);
    color: #fff;
    font-size: 11px;
    font-weight: 600;
    line-height: 1.5;
    letter-spacing: 0.02em;
    text-transform: lowercase;
    white-space: nowrap;
  }
</style>
