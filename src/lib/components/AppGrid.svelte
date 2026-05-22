<script lang="ts">
  import { visibleApps } from "$lib/stores";
  import AppCard from "./AppCard.svelte";
</script>

<div class="grid" role="list">
  {#each $visibleApps as app (app.uid)}
    <div role="listitem">
      <AppCard {app} />
    </div>
  {/each}
</div>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: 14px;
    padding: 18px;
  }

  .grid > :global(div) {
    display: flex;
  }
</style>
