<script lang="ts">
  import type { App } from "$lib/types";
  import { iconSrc } from "$lib/api";
  import { selected } from "$lib/stores";
  import SourceBadge from "./SourceBadge.svelte";

  let { app }: { app: App } = $props();

  let src = $derived(iconSrc(app));
  let failed = $state(false);
  let isSelected = $derived($selected?.uid === app.uid);

  // Reset the error flag when the underlying icon changes.
  $effect(() => {
    src;
    failed = false;
  });

  let initial = $derived(app.name.trim().charAt(0).toUpperCase() || "?");

  function open() {
    selected.set(app);
  }
</script>

<button
  type="button"
  class="card"
  class:selected={isSelected}
  aria-pressed={isSelected}
  title={app.name}
  onclick={open}
>
  <div class="icon">
    {#if src && !failed}
      <img
        src={src}
        alt=""
        loading="lazy"
        onerror={() => (failed = true)}
      />
    {:else}
      <span class="fallback" aria-hidden="true">{initial}</span>
    {/if}
  </div>

  <div class="name">{app.name}</div>

  <div class="meta">
    <SourceBadge source={app.source} />
    {#if app.version}
      <span class="version">{app.version}</span>
    {/if}
  </div>
</button>

<style>
  .card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 16px 12px 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    box-shadow: var(--shadow-sm);
    cursor: pointer;
    font-family: inherit;
    text-align: center;
    color: var(--text);
    width: 100%;
  }

  .card:hover {
    transform: translateY(-2px);
    border-color: var(--accent);
    box-shadow: var(--shadow-md);
  }

  .card:active {
    transform: translateY(0);
  }

  .card.selected {
    border-color: var(--accent);
    box-shadow:
      var(--shadow-md),
      inset 0 0 0 1px var(--accent);
  }

  .icon {
    width: 56px;
    height: 56px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .icon img {
    width: 56px;
    height: 56px;
    object-fit: contain;
    image-rendering: auto;
  }

  .fallback {
    width: 56px;
    height: 56px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 14px;
    background: var(--surface-hover);
    border: 1px solid var(--border);
    color: var(--muted);
    font-size: 26px;
    font-weight: 600;
  }

  .name {
    font-size: 13px;
    font-weight: 600;
    line-height: 1.3;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    text-overflow: ellipsis;
    /* Reserve two lines so cards stay the same height. */
    min-height: calc(2 * 1.3em);
    width: 100%;
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
    justify-content: center;
    max-width: 100%;
  }

  .version {
    font-size: 11px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 90px;
  }
</style>
