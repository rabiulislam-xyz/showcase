<script lang="ts">
  import "@fontsource/inter/400.css";
  import "@fontsource/inter/500.css";
  import "@fontsource/inter/600.css";
  import "@fontsource/source-serif-4/400.css";
  import "@fontsource/source-serif-4/500.css";
  import "@fontsource/source-serif-4/600.css";
  import "@fontsource/jetbrains-mono/400.css";
  import "@fontsource/jetbrains-mono/500.css";
  import "$lib/theme.css";
  import { onMount } from "svelte";
  import { initTheme } from "$lib/stores";

  let { children } = $props();

  onMount(initTheme);
</script>

{@render children()}
