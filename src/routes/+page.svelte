<script lang="ts">
  import { onMount } from "svelte";
  import {
    status,
    errorMsg,
    visibleApps,
    apps,
    loadApps,
    query,
    sourceFilter,
  } from "$lib/stores";
  import Header from "$lib/components/Header.svelte";
  import StatusBanner from "$lib/components/StatusBanner.svelte";
  import AppGrid from "$lib/components/AppGrid.svelte";
  import AppDetail from "$lib/components/AppDetail.svelte";
  import Spinner from "$lib/components/Spinner.svelte";
  import Toast from "$lib/components/Toast.svelte";

  onMount(loadApps);

  // Distinguish "nothing installed" from "filters hide everything".
  let hasApps = $derived($apps.length > 0);
  let filtersActive = $derived($query.trim() !== "" || $sourceFilter !== "all");

  function clearFilters() {
    query.set("");
    sourceFilter.set("all");
  }
</script>

<div class="app">
  <Header />
  <StatusBanner />

  <main class="region">
    {#if $status === "loading"}
      <div class="centered">
        <Spinner />
        <p class="muted">Loading apps…</p>
      </div>
    {:else if $status === "error"}
      <div class="centered">
        <div class="panel">
          <h2>Couldn’t load apps</h2>
          <p class="muted">{$errorMsg || "An unexpected error occurred."}</p>
          <button type="button" class="retry" onclick={() => loadApps()}>
            Retry
          </button>
        </div>
      </div>
    {:else if $visibleApps.length === 0}
      <div class="centered">
        <div class="empty">
          {#if hasApps && filtersActive}
            <p>No apps match your filters.</p>
            <button type="button" class="ghost" onclick={clearFilters}>
              Clear filters
            </button>
          {:else}
            <p>No apps found.</p>
          {/if}
        </div>
      </div>
    {:else}
      <AppGrid />
    {/if}
  </main>
</div>

<AppDetail />
<Toast />

<style>
  .app {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
  }

  .region {
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  .centered {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 14px;
    padding: 40px 20px;
    text-align: center;
  }

  .muted {
    color: var(--muted);
    margin: 0;
  }

  .panel,
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    max-width: 360px;
  }

  .panel h2 {
    margin: 0;
    font-size: 17px;
  }

  .retry,
  .ghost {
    padding: 8px 16px;
    border-radius: var(--radius-ctrl);
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }

  .retry {
    border: none;
    background: var(--accent);
    color: #fff;
  }
  .retry:hover {
    background: var(--accent-hover);
  }

  .ghost {
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text);
  }
  .ghost:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
</style>
