<script lang="ts">
  let { size = 28 }: { size?: number } = $props();
</script>

<span
  class="spinner"
  role="status"
  aria-label="Loading"
  style="--size: {size}px"
></span>

<style>
  .spinner {
    display: inline-block;
    width: var(--size);
    height: var(--size);
    border: 3px solid color-mix(in srgb, var(--accent) 22%, transparent);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
