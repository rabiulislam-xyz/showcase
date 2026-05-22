<script lang="ts">
  import { warnings } from "$lib/stores";

  let dismissed = $state(false);
  let show = $derived($warnings.length > 0 && !dismissed);

  // A fresh set of warnings (e.g. after refresh) should re-surface the strip.
  $effect(() => {
    $warnings;
    dismissed = false;
  });
</script>

{#if show}
  <div class="banner" role="alert">
    <svg class="warn-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/></svg>

    <div class="body">
      <strong>Some sources had issues</strong>
      <ul>
        {#each $warnings as w, i (i)}
          <li>{w}</li>
        {/each}
      </ul>
    </div>

    <button class="dismiss" aria-label="Dismiss warnings" onclick={() => (dismissed = true)}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
    </button>
  </div>
{/if}

<style>
  .banner {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    margin: 0 0 20px;
    padding: 12px 14px;
    background: var(--apt-tint);
    border: 1px solid color-mix(in oklab, var(--apt) 30%, transparent);
    border-radius: var(--radius-ctrl);
    color: var(--text);
    font-size: 13px;
  }
  .warn-icon {
    flex: none;
    color: var(--apt);
    margin-top: 1px;
  }
  .body {
    flex: 1;
    min-width: 0;
  }
  .body strong {
    font-weight: 500;
  }
  .body ul {
    margin: 4px 0 0;
    padding-left: 18px;
    color: var(--text-muted);
  }
  .body li {
    word-break: break-word;
  }
  .dismiss {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: var(--radius-sm);
    color: var(--text-faint);
    transition: all 150ms var(--ease);
  }
  .dismiss:hover {
    background: color-mix(in oklab, var(--apt) 14%, transparent);
    color: var(--text);
  }
</style>
