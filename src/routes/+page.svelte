<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  type App = { uid: string; name: string; source: string; version: string | null };
  let apps: App[] = $state([]);
  let error = $state("");

  onMount(async () => {
    try { apps = await invoke<App[]>("list_apps"); }
    catch (e) { error = String(e); }
  });
</script>

<main style="padding:1rem;font-family:system-ui">
  <h1>Showcase — {apps.length} apps</h1>
  {#if error}<p style="color:red">{error}</p>{/if}
  <ul>
    {#each apps as a (a.uid)}
      <li>{a.name} <small style="opacity:.6">({a.source} {a.version ?? ""})</small></li>
    {/each}
  </ul>
</main>
