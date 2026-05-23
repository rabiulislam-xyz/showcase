<script lang="ts">
  import { onMount } from "svelte";
  import {
    status,
    errorMsg,
    visibleApps,
    loadApps,
    query,
    sourceFilter,
    selected,
  } from "$lib/stores";
  import Header from "$lib/components/Header.svelte";
  import StatusBanner from "$lib/components/StatusBanner.svelte";
  import AppGrid from "$lib/components/AppGrid.svelte";
  import AppDetail from "$lib/components/AppDetail.svelte";
  import SkeletonCard from "$lib/components/SkeletonCard.svelte";
  import Toast from "$lib/components/Toast.svelte";

  onMount(loadApps);

  const SKELETON_COUNT = 10;

  function clearFilters() {
    query.set("");
    sourceFilter.set("all");
  }
</script>

<!-- While the detail drawer is open it owns focus; mark the rest inert so Tab
     can't reach the background grid and assistive tech skips it. -->
<div inert={$selected !== null}>
  <Header />

  <main>
    <StatusBanner />

  {#if $status === "loading"}
    <div class="grid">
      {#each Array(SKELETON_COUNT) as _, i (i)}
        <SkeletonCard />
      {/each}
    </div>
  {:else if $status === "error"}
    <div class="error-panel">
      <div class="error-icon" aria-hidden="true">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 8v4"/><path d="M12 16h.01"/></svg>
      </div>
      <h3>Couldn’t load apps</h3>
      <p>{$errorMsg || "An unexpected error occurred while scanning your system."}</p>
      <button class="reset" onclick={() => loadApps()}>Retry</button>
    </div>
  {:else if $visibleApps.length === 0}
    <div class="empty">
      <div class="empty-icon" aria-hidden="true">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></svg>
      </div>
      <h3>No apps match your filters</h3>
      <p>Try a different search term, or clear the source filter to see everything installed on your system.</p>
      <button class="reset" onclick={clearFilters}>Clear filters</button>
    </div>
  {:else}
    <AppGrid />
  {/if}
  </main>
</div>

<AppDetail />
<Toast />

<style>
  main {
    max-width: 1400px;
    margin: 0 auto;
    padding: 32px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 16px;
  }

  .empty,
  .error-panel {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 80px 32px;
    text-align: center;
    color: var(--text-muted);
  }
  .empty-icon,
  .error-icon {
    width: 56px;
    height: 56px;
    border-radius: 16px;
    background: var(--surface-2);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-faint);
    margin-bottom: 20px;
  }
  .error-icon {
    color: var(--destructive);
    background: var(--destructive-tint);
  }
  .empty h3,
  .error-panel h3 {
    font-family: var(--serif);
    font-weight: 500;
    font-size: 22px;
    color: var(--text);
    margin: 0 0 6px;
    letter-spacing: -0.01em;
  }
  .empty p,
  .error-panel p {
    margin: 0;
    max-width: 380px;
    word-break: break-word;
  }
  .reset {
    margin-top: 18px;
    padding: 8px 16px;
    border-radius: var(--radius-ctrl);
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
    transition: all 150ms var(--ease);
  }
  .reset:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  @media (max-width: 720px) {
    main {
      padding: 24px 20px;
    }
  }
</style>
