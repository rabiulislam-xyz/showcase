<script lang="ts">
  import { toasts, dismissToast } from "$lib/stores";
</script>

<div class="toast-region" aria-live="polite" aria-atomic="false">
  {#each $toasts as toast (toast.id)}
    <div class="toast" class:success={toast.kind === "success"} class:error={toast.kind === "error"}>
      <span class="toast-msg">{toast.msg}</span>
      <button
        type="button"
        class="toast-dismiss"
        aria-label="Dismiss notification"
        onclick={() => dismissToast(toast.id)}
      >
        ×
      </button>
    </div>
  {/each}
</div>

<style>
  .toast-region {
    position: fixed;
    bottom: 24px;
    right: 24px;
    z-index: 80;
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: min(360px, calc(100% - 32px));
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
    border-radius: var(--radius-ctrl);
    box-shadow: var(--shadow-md);
    font-size: 13px;
    font-weight: 500;
    pointer-events: all;
    border-left: 4px solid transparent;
  }

  .toast.success {
    background: var(--surface);
    border-left-color: #26a269;
    color: var(--text);
  }

  .toast.error {
    background: var(--surface);
    border-left-color: var(--destructive);
    color: var(--text);
  }

  .toast-msg {
    flex: 1;
    line-height: 1.4;
  }

  .toast-dismiss {
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
    color: var(--muted);
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
  }
  .toast-dismiss:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
</style>
