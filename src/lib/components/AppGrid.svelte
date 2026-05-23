<script lang="ts">
  import { visibleApps } from "$lib/stores";
  import AppCard from "./AppCard.svelte";
</script>

<div class="app-grid">
  {#each $visibleApps as app (app.uid)}
    <AppCard {app} />
  {/each}
</div>
