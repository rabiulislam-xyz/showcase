<script lang="ts">
  import { toasts, dismissToast } from "$lib/stores";

  const titleFor = (kind: "success" | "error") =>
    kind === "success" ? "Done" : "Something went wrong";
</script>

<div class="toast-stack" aria-live="polite" aria-atomic="false">
  {#each $toasts as toast (toast.id)}
    <div
      class="toast"
      class:success={toast.kind === "success"}
      class:error={toast.kind === "error"}
      role={toast.kind === "error" ? "alert" : undefined}
    >
      <div class="toast-icon" aria-hidden="true">
        {#if toast.kind === "success"}
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>
        {:else}
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
        {/if}
      </div>
      <div class="toast-body">
        <div class="toast-title">{titleFor(toast.kind)}</div>
        <div class="toast-msg">{toast.msg}</div>
      </div>
      <button
        class="toast-close"
        aria-label="Dismiss notification"
        onclick={() => dismissToast(toast.id)}
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
      </button>
    </div>
  {/each}
</div>

<style>
  .toast-stack {
    position: fixed;
    bottom: 24px;
    right: 24px;
    z-index: 300;
    display: flex;
    flex-direction: column;
    gap: 10px;
    pointer-events: none;
  }
  .toast {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 12px 16px 12px 14px;
    box-shadow: var(--shadow);
    display: flex;
    align-items: flex-start;
    gap: 10px;
    min-width: 280px;
    max-width: 380px;
    pointer-events: auto;
    transform: translateX(20px);
    opacity: 0;
    animation: toast-in 220ms var(--ease) forwards;
  }
  @keyframes toast-in {
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }
  .toast-icon {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    margin-top: 1px;
  }
  .toast.success .toast-icon {
    background: rgba(78, 131, 127, 0.15);
    color: var(--flatpak);
  }
  .toast.error .toast-icon {
    background: var(--destructive-tint);
    color: var(--destructive);
  }
  .toast-body {
    flex: 1;
    min-width: 0;
  }
  .toast-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
    margin-bottom: 2px;
  }
  .toast-msg {
    font-size: 12px;
    color: var(--text-muted);
    line-height: 1.45;
    word-break: break-word;
  }
  .toast-close {
    color: var(--text-faint);
    padding: 2px;
    border-radius: 4px;
    transition: color 150ms var(--ease);
    flex-shrink: 0;
  }
  .toast-close:hover {
    color: var(--text);
  }
</style>
