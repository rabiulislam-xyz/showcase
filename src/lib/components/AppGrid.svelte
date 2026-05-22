<script lang="ts">
  import { visibleApps } from "$lib/stores";
  import AppCard from "./AppCard.svelte";
</script>

<div class="grid">
  {#each $visibleApps as app (app.uid)}
    <AppCard {app} />
  {/each}
</div>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 16px;
    align-items: stretch;
  }
</style>
