<script lang="ts">
  import type { App } from "$lib/types";
  import { iconSrc } from "$lib/api";
  import { selected } from "$lib/stores";
  import { tileColor, tileInitial, sourceLabel } from "$lib/avatar";

  let { app }: { app: App } = $props();

  let src = $derived(iconSrc(app));
  let failed = $state(false);
  let isSelected = $derived($selected?.uid === app.uid);

  // Reset the error flag when the underlying icon changes.
  $effect(() => {
    src;
    failed = false;
  });

  let initial = $derived(tileInitial(app.name));
  let tint = $derived(tileColor(app.name));

  function open() {
    selected.set(app);
  }
</script>

<button
  class="card"
  class:selected={isSelected}
  aria-pressed={isSelected}
  aria-label="{app.name} — open details"
  onclick={open}
>
  {#if src && !failed}
    <img
      class="card-icon-img"
      src={src}
      alt=""
      loading="lazy"
      onerror={() => (failed = true)}
    />
  {:else}
    <div class="card-icon" style="background:{tint}" aria-hidden="true">{initial}</div>
  {/if}

  <div class="card-name">{app.name}</div>

  <div class="card-meta">
    <span class="tag {app.source}">{sourceLabel(app.source)}</span>
    {#if app.version}
      <span class="card-version">{app.version}</span>
    {/if}
  </div>
</button>

<style>
  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    padding: 18px 16px 16px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
    cursor: pointer;
    transition: transform 150ms var(--ease), border-color 150ms var(--ease),
      box-shadow 150ms var(--ease);
    text-align: left;
    width: 100%;
    height: 100%;
    position: relative;
  }
  .card:hover {
    transform: translateY(-2px);
    border-color: var(--accent);
    box-shadow: 0 4px 12px rgba(217, 119, 87, 0.1);
  }
  .card.selected {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }

  .card-icon,
  .card-icon-img {
    width: 56px;
    height: 56px;
    border-radius: 14px;
    flex-shrink: 0;
  }
  .card-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--serif);
    font-size: 26px;
    font-weight: 500;
    color: #fff;
    letter-spacing: -0.02em;
  }
  .card-icon-img {
    object-fit: contain;
  }

  .card-name {
    font-weight: 500;
    font-size: 14px;
    line-height: 1.35;
    color: var(--text);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    word-break: break-word;
  }

  .card-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: auto;
  }
  .card-version {
    font-size: 12px;
    color: var(--text-faint);
    font-variant-numeric: tabular-nums;
  }

  .tag {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    font-weight: 500;
    padding: 3px 8px;
    border-radius: 5px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-variant-numeric: tabular-nums;
  }
  .tag.apt {
    color: var(--apt);
    background: var(--apt-tint);
  }
  .tag.flatpak {
    color: var(--flatpak);
    background: var(--flatpak-tint);
  }
  .tag.snap {
    color: var(--snap);
    background: var(--snap-tint);
  }
</style>
