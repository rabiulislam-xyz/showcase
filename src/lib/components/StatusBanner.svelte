<script lang="ts">
  import { warnings } from "$lib/stores";
  import { slide } from "svelte/transition";

  let dismissed = $state(false);
  let show = $derived($warnings.length > 0 && !dismissed);

  // A fresh set of warnings (e.g. after refresh) should re-surface the banner.
  $effect(() => {
    $warnings;
    dismissed = false;
  });
</script>

{#if show}
  <div class="banner" role="alert" transition:slide={{ duration: 150 }}>
    <svg class="warn-icon" viewBox="0 0 16 16" aria-hidden="true">
      <path
        d="M8 1.5a.75.75 0 0 1 .66.39l6.25 11.5A.75.75 0 0 1 14.25 14.5H1.75a.75.75 0 0 1-.66-1.11l6.25-11.5A.75.75 0 0 1 8 1.5Zm0 3.75a.75.75 0 0 0-.75.75v3a.75.75 0 0 0 1.5 0v-3A.75.75 0 0 0 8 5.25Zm0 6.75a.9.9 0 1 0 0-1.8.9.9 0 0 0 0 1.8Z"
        fill="currentColor"
      />
    </svg>

    <div class="body">
      <strong>Some sources had issues:</strong>
      <ul>
        {#each $warnings as w, i (i)}
          <li>{w}</li>
        {/each}
      </ul>
    </div>

    <button
      type="button"
      class="dismiss"
      aria-label="Dismiss warnings"
      onclick={() => (dismissed = true)}
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path
          d="M4.22 4.22a.75.75 0 0 1 1.06 0L8 6.94l2.72-2.72a.75.75 0 1 1 1.06 1.06L9.06 8l2.72 2.72a.75.75 0 1 1-1.06 1.06L8 9.06l-2.72 2.72a.75.75 0 0 1-1.06-1.06L6.94 8 4.22 5.28a.75.75 0 0 1 0-1.06Z"
          fill="currentColor"
        />
      </svg>
    </button>
  </div>
{/if}

<style>
  .banner {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    margin: 12px 18px 0;
    padding: 10px 12px;
    background: var(--warning-bg);
    border: 1px solid var(--warning-border);
    border-radius: var(--radius-ctrl);
    color: var(--warning-text);
    font-size: 13px;
  }

  .warn-icon {
    flex: none;
    width: 16px;
    height: 16px;
    margin-top: 1px;
  }

  .body {
    flex: 1;
    min-width: 0;
  }

  .body ul {
    margin: 4px 0 0;
    padding-left: 18px;
  }

  .body li {
    word-break: break-word;
  }

  .dismiss {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    padding: 0;
    border: none;
    border-radius: var(--radius-pill);
    background: transparent;
    color: inherit;
    cursor: pointer;
  }
  .dismiss:hover {
    background: color-mix(in srgb, var(--warning-text) 14%, transparent);
  }
  .dismiss svg {
    width: 14px;
    height: 14px;
  }
</style>
