<script lang="ts">
  import {
    query,
    sourceFilter,
    sortKey,
    apps,
    visibleApps,
    loadApps,
    status,
  } from "$lib/stores";
  import type { Source } from "$lib/types";
  import type { SortKey } from "$lib/filter";

  type FilterKey = "all" | Source;

  let counts = $derived.by(() => {
    const c = { all: $apps.length, apt: 0, flatpak: 0, snap: 0 };
    for (const a of $apps) c[a.source] += 1;
    return c;
  });

  const filters: { key: FilterKey; label: string }[] = [
    { key: "all", label: "All" },
    { key: "apt", label: "apt" },
    { key: "flatpak", label: "flatpak" },
    { key: "snap", label: "snap" },
  ];

  const sorts: { value: SortKey; label: string }[] = [
    { value: "name", label: "Name" },
    { value: "size", label: "Size" },
    { value: "recent", label: "Recent" },
  ];

  let refreshing = $derived($status === "loading");
</script>

<header class="header">
  <div class="brand">
    <h1>Showcase</h1>
    <span class="count">{$visibleApps.length} apps</span>
  </div>

  <div class="search">
    <svg class="search-icon" viewBox="0 0 16 16" aria-hidden="true">
      <path
        d="M7 1a6 6 0 0 1 4.74 9.68l3.29 3.29a.75.75 0 0 1-1.06 1.06l-3.29-3.29A6 6 0 1 1 7 1Zm0 1.5a4.5 4.5 0 1 0 0 9 4.5 4.5 0 0 0 0-9Z"
        fill="currentColor"
      />
    </svg>
    <input
      type="search"
      placeholder="Search apps…"
      aria-label="Search apps"
      bind:value={$query}
    />
    {#if $query}
      <button
        type="button"
        class="clear"
        aria-label="Clear search"
        onclick={() => query.set("")}
      >
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path
            d="M4.22 4.22a.75.75 0 0 1 1.06 0L8 6.94l2.72-2.72a.75.75 0 1 1 1.06 1.06L9.06 8l2.72 2.72a.75.75 0 1 1-1.06 1.06L8 9.06l-2.72 2.72a.75.75 0 0 1-1.06-1.06L6.94 8 4.22 5.28a.75.75 0 0 1 0-1.06Z"
            fill="currentColor"
          />
        </svg>
      </button>
    {/if}
  </div>

  <div class="segmented" role="group" aria-label="Filter by source">
    {#each filters as f (f.key)}
      <button
        type="button"
        class="seg"
        class:active={$sourceFilter === f.key}
        aria-pressed={$sourceFilter === f.key}
        onclick={() => sourceFilter.set(f.key)}
      >
        {f.label}
        <span class="seg-count">{counts[f.key]}</span>
      </button>
    {/each}
  </div>

  <label class="sort">
    <span class="visually-hidden">Sort by</span>
    <select bind:value={$sortKey} aria-label="Sort apps">
      {#each sorts as s (s.value)}
        <option value={s.value}>{s.label}</option>
      {/each}
    </select>
  </label>

  <button
    type="button"
    class="refresh"
    class:spinning={refreshing}
    aria-label="Refresh app list"
    title="Refresh"
    disabled={refreshing}
    onclick={() => loadApps()}
  >
    <svg viewBox="0 0 16 16" aria-hidden="true">
      <path
        d="M8 2.5a5.5 5.5 0 1 0 5.32 4.13.75.75 0 0 1 1.45-.38A7 7 0 1 1 8 1a6.97 6.97 0 0 1 4.5 1.64V1.75a.75.75 0 0 1 1.5 0V4.5a.75.75 0 0 1-.75.75H10.5a.75.75 0 0 1 0-1.5h1.27A5.48 5.48 0 0 0 8 2.5Z"
        fill="currentColor"
      />
    </svg>
  </button>
</header>

<style>
  .header {
    position: sticky;
    top: 0;
    z-index: 10;
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
    padding: 12px 18px;
    background: var(--header-bg);
    backdrop-filter: saturate(1.4) blur(12px);
    -webkit-backdrop-filter: saturate(1.4) blur(12px);
    border-bottom: 1px solid var(--border);
  }

  .brand {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }

  .brand h1 {
    margin: 0;
    font-size: 18px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }

  .count {
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
  }

  .search {
    position: relative;
    display: flex;
    align-items: center;
    flex: 1 1 220px;
    min-width: 180px;
  }

  .search-icon {
    position: absolute;
    left: 10px;
    width: 15px;
    height: 15px;
    color: var(--muted);
    pointer-events: none;
  }

  .search input {
    width: 100%;
    padding: 7px 32px 7px 32px;
    border: 1px solid var(--border);
    border-radius: var(--radius-ctrl);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: 13px;
  }

  .search input:focus {
    border-color: var(--accent);
    outline: none;
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 28%, transparent);
  }

  /* Hide the native search clear control; we provide our own. */
  .search input::-webkit-search-cancel-button {
    appearance: none;
  }

  .clear {
    position: absolute;
    right: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    padding: 0;
    border: none;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }
  .clear:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .clear svg {
    width: 13px;
    height: 13px;
  }

  .segmented {
    display: inline-flex;
    padding: 2px;
    background: var(--surface-hover);
    border: 1px solid var(--border);
    border-radius: var(--radius-ctrl);
  }

  .seg {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 10px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .seg:hover {
    color: var(--text);
  }
  .seg.active {
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-sm);
  }

  .seg-count {
    font-size: 11px;
    font-weight: 600;
    padding: 0 5px;
    border-radius: var(--radius-pill);
    background: color-mix(in srgb, var(--muted) 18%, transparent);
    color: var(--muted);
  }
  .seg.active .seg-count {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    color: var(--accent);
  }

  .sort select {
    padding: 7px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-ctrl);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .sort select:focus {
    border-color: var(--accent);
    outline: none;
  }

  .refresh {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    border: 1px solid var(--border);
    border-radius: var(--radius-ctrl);
    background: var(--surface);
    color: var(--text);
    cursor: pointer;
  }
  .refresh:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
  }
  .refresh:disabled {
    cursor: default;
    opacity: 0.7;
  }
  .refresh svg {
    width: 16px;
    height: 16px;
  }
  .refresh.spinning svg {
    animation: spin 0.8s linear infinite;
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
