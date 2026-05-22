<script lang="ts">
  import Spinner from "./Spinner.svelte";

  let {
    open,
    title,
    message,
    confirmLabel = "Confirm",
    destructive = false,
    busy = false,
    onconfirm,
    oncancel,
  }: {
    open: boolean;
    title: string;
    message: string;
    confirmLabel?: string;
    destructive?: boolean;
    busy?: boolean;
    onconfirm: () => void;
    oncancel: () => void;
  } = $props();

  let confirmBtn = $state<HTMLButtonElement | undefined>();

  // Focus the confirm button whenever the dialog opens.
  $effect(() => {
    if (open && confirmBtn) {
      confirmBtn.focus();
    }
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && !busy) {
      oncancel();
    }
  }

  function handleBackdrop() {
    if (!busy) oncancel();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <!-- Backdrop -->
  <div
    class="backdrop"
    aria-hidden="true"
    onclick={handleBackdrop}
  ></div>

  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="cd-title"
    aria-describedby="cd-message"
  >
    <h2 id="cd-title" class="title">{title}</h2>
    <p id="cd-message" class="message">{message}</p>

    <div class="actions">
      <button
        type="button"
        class="btn-cancel"
        disabled={busy}
        onclick={oncancel}
      >
        Cancel
      </button>
      <button
        type="button"
        class="btn-confirm"
        class:destructive
        disabled={busy}
        bind:this={confirmBtn}
        onclick={onconfirm}
      >
        {#if busy}
          <Spinner size={16} />
        {/if}
        {confirmLabel}
      </button>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 60;
    background: rgba(0, 0, 0, 0.45);
  }

  .dialog {
    position: fixed;
    z-index: 61;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: min(420px, calc(100% - 32px));
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    box-shadow: var(--shadow-lg);
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .title {
    margin: 0;
    font-size: 17px;
    font-weight: 700;
    color: var(--text);
    line-height: 1.3;
  }

  .message {
    margin: 0;
    font-size: 14px;
    color: var(--muted);
    line-height: 1.55;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }

  .btn-cancel,
  .btn-confirm {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    border-radius: var(--radius-ctrl);
    font: inherit;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition:
      background-color var(--transition),
      border-color var(--transition),
      color var(--transition),
      opacity var(--transition);
  }

  .btn-cancel {
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text);
  }
  .btn-cancel:hover:not(:disabled) {
    background: var(--surface-hover);
  }

  .btn-confirm {
    border: 1px solid var(--accent);
    background: var(--accent);
    color: #fff;
  }
  .btn-confirm:hover:not(:disabled) {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }

  .btn-confirm.destructive {
    border-color: var(--destructive);
    background: var(--destructive);
    color: #fff;
  }
  .btn-confirm.destructive:hover:not(:disabled) {
    /* Darken destructive slightly on hover */
    filter: brightness(0.9);
  }

  .btn-cancel:disabled,
  .btn-confirm:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
</style>
