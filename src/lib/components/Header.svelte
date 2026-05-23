<script lang="ts">
  import {
    query,
    sourceFilter,
    sortKey,
    sortDir,
    categoryFilter,
    availableCats,
    apps,
    loadApps,
    status,
    theme,
    toggleTheme,
    selected,
    setSort,
    toggleSortDir,
  } from "$lib/stores";
  import type { Source } from "$lib/types";
  import type { SortKey } from "$lib/filter";
  import Dropdown from "./Dropdown.svelte";

  type FilterKey = "all" | Source;

  let counts = $derived.by(() => {
    const c = { all: $apps.length, apt: 0, flatpak: 0, snap: 0 };
    for (const a of $apps) c[a.source] += 1;
    return c;
  });

  const filters: { key: FilterKey; label: string; dot?: Source }[] = [
    { key: "all", label: "All" },
    { key: "apt", label: "APT", dot: "apt" },
    { key: "flatpak", label: "Flatpak", dot: "flatpak" },
    { key: "snap", label: "Snap", dot: "snap" },
  ];

  const sortOptions: { value: SortKey; label: string }[] = [
    { value: "name", label: "Name" },
    { value: "size", label: "Size" },
    { value: "recent", label: "Recently installed" },
  ];

  let categoryOptions = $derived([
    { value: "all", label: "All categories" },
    ...$availableCats.map((c) => ({ value: c, label: c })),
  ]);

  let refreshing = $derived($status === "loading");
  // Drives the one-shot spin animation on refresh click.
  let spinning = $state(false);

  function refresh() {
    spinning = false;
    // Force reflow so the animation restarts on every click.
    requestAnimationFrame(() => {
      spinning = true;
    });
    loadApps();
  }

  // --- Cmd/Ctrl+K to focus the search input ---
  let searchInput = $state<HTMLInputElement | undefined>();

  function onWindowKeydown(e: KeyboardEvent) {
    if ($selected !== null) return;
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
      e.preventDefault();
      searchInput?.focus();
      searchInput?.select();
    }
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

<header class="header">
  <div class="header-inner">
    <div class="brand">
      <span class="wordmark">Showcase<span class="dot"></span></span>
      <span class="app-count"><strong>{$apps.length}</strong> apps installed</span>
    </div>

    <div class="search" role="search">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></svg>
      <input
        bind:this={searchInput}
        type="text"
        placeholder="Search applications…"
        autocomplete="off"
        spellcheck="false"
        aria-label="Search applications"
        bind:value={$query}
      />
      <kbd>Ctrl K</kbd>
    </div>

    <div class="actions">
      <button
        class="icon-btn"
        class:spinning
        title="Refresh"
        aria-label="Refresh app list"
        disabled={refreshing}
        onclick={refresh}
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 12a9 9 0 0 1 15.5-6.3L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-15.5 6.3L3 16"/><path d="M3 21v-5h5"/></svg>
      </button>
      <button
        class="icon-btn"
        title="Toggle theme"
        aria-label="Toggle theme"
        onclick={toggleTheme}
      >
        {#if $theme === "dark"}
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/></svg>
        {:else}
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>
        {/if}
      </button>
    </div>

    <div class="filter-row">
      <div class="filter-left">
        <div class="segmented" role="group" aria-label="Filter by source">
          {#each filters as f (f.key)}
            <button
              class="seg-btn"
              class:active={$sourceFilter === f.key}
              aria-pressed={$sourceFilter === f.key}
              onclick={() => sourceFilter.set(f.key)}
            >
              {#if f.dot}<span class="seg-dot {f.dot}"></span>{/if}
              <span>{f.label}</span>
              <span class="count">{counts[f.key]}</span>
            </button>
          {/each}
        </div>

        <Dropdown
          value={$categoryFilter}
          options={categoryOptions}
          onSelect={(v) => categoryFilter.set(v)}
          ariaLabel="Filter by category"
        />
      </div>

      <div class="sort">
        <span>Sort by</span>
        <Dropdown
          value={$sortKey}
          options={sortOptions}
          onSelect={(v) => setSort(v as SortKey)}
          ariaLabel="Sort by"
        />
        <button
          class="icon-btn sort-dir-btn"
          aria-label={$sortDir === "asc" ? "Sort ascending" : "Sort descending"}
          title={$sortDir === "asc" ? "Sort ascending" : "Sort descending"}
          onclick={toggleSortDir}
        >
          {#if $sortDir === "asc"}
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 19V5"/><path d="m5 12 7-7 7 7"/></svg>
          {:else}
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 5v14"/><path d="m19 12-7 7-7-7"/></svg>
          {/if}
        </button>
      </div>
    </div>
  </div>
</header>

<style>
  .header {
    position: sticky;
    top: 0;
    z-index: 50;
    background: color-mix(in oklab, var(--bg) 92%, transparent);
    backdrop-filter: saturate(160%) blur(12px);
    -webkit-backdrop-filter: saturate(160%) blur(12px);
    border-bottom: 1px solid var(--border);
    transition: background-color 200ms var(--ease), border-color 200ms var(--ease);
  }
  .header-inner {
    max-width: 1400px;
    margin: 0 auto;
    padding: 18px 32px;
    display: grid;
    grid-template-columns: auto 1fr auto;
    grid-template-areas:
      "brand search actions"
      "filters filters filters";
    gap: 16px 24px;
    align-items: center;
  }
  .brand {
    grid-area: brand;
    display: flex;
    align-items: baseline;
    gap: 12px;
  }
  .wordmark {
    font-family: var(--serif);
    font-size: 28px;
    font-weight: 500;
    letter-spacing: -0.015em;
    color: var(--text);
  }
  .wordmark .dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    background: var(--accent);
    border-radius: 50%;
    vertical-align: middle;
    margin-left: 2px;
    margin-bottom: 4px;
  }
  .app-count {
    font-size: 13px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .app-count strong {
    color: var(--text);
    font-weight: 500;
  }

  .search {
    grid-area: search;
    position: relative;
    max-width: 480px;
    width: 100%;
    justify-self: center;
  }
  .search input {
    width: 100%;
    height: 40px;
    padding: 0 14px 0 40px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-ctrl);
    font-size: 14px;
    color: var(--text);
    transition: all 150ms var(--ease);
  }
  .search input::placeholder {
    color: var(--text-faint);
  }
  .search input:hover {
    border-color: var(--border-strong);
  }
  .search input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-tint);
    outline: none;
  }
  .search svg {
    position: absolute;
    left: 14px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-muted);
    pointer-events: none;
  }
  .search kbd {
    position: absolute;
    right: 10px;
    top: 50%;
    transform: translateY(-50%);
    font-family: var(--mono);
    font-size: 11px;
    color: var(--text-faint);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 2px 6px;
    pointer-events: none;
  }

  .actions {
    grid-area: actions;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .icon-btn {
    width: 40px;
    height: 40px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-ctrl);
    color: var(--text-muted);
    border: 1px solid transparent;
    transition: all 150ms var(--ease);
  }
  .icon-btn:hover {
    background: var(--surface-2);
    color: var(--text);
    border-color: var(--border);
  }
  .icon-btn:disabled {
    cursor: default;
    opacity: 0.6;
  }
  .icon-btn.spinning svg {
    animation: spin 600ms var(--ease);
  }
  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .filter-row {
    grid-area: filters;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }
  .segmented {
    display: inline-flex;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-ctrl);
    padding: 3px;
    gap: 2px;
  }
  .seg-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 14px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-muted);
    border-radius: 6px;
    transition: all 150ms var(--ease);
    position: relative;
  }
  .seg-btn:hover:not(.active) {
    color: var(--text);
  }
  .seg-btn.active {
    background: var(--surface);
    color: var(--text);
    box-shadow: 0 1px 2px rgba(20, 20, 19, 0.06);
  }
  :global([data-theme="dark"]) .seg-btn.active {
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
  }
  .seg-btn .count {
    font-variant-numeric: tabular-nums;
    font-size: 12px;
    color: var(--text-faint);
    font-weight: 400;
  }
  .seg-btn.active .count {
    color: var(--text-muted);
  }
  .seg-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }
  .seg-dot.apt {
    background: var(--apt);
  }
  .seg-dot.flatpak {
    background: var(--flatpak);
  }
  .seg-dot.snap {
    background: var(--snap);
  }

  .filter-left {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .sort {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
    font-size: 13px;
  }
  /* Smaller than the header icon-btns (40px) to sit flush next to the dropdown */
  .sort-dir-btn {
    width: 32px;
    height: 32px;
  }

  @media (max-width: 720px) {
    .header-inner {
      grid-template-columns: 1fr;
      grid-template-areas: "brand" "search" "actions" "filters";
      padding: 16px 20px;
    }
    .actions {
      justify-content: flex-end;
    }
    .filter-row {
      flex-direction: column;
      align-items: stretch;
    }
  }
</style>
