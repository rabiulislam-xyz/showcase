<script lang="ts">
  import { humanSize } from "$lib/format";
  import Spinner from "./Spinner.svelte";

  let {
    open,
    name,
    sizeBytes,
    aptNote = false,
    busy = false,
    onconfirm,
    oncancel,
  }: {
    open: boolean;
    name: string;
    sizeBytes: number | null;
    aptNote?: boolean;
    busy?: boolean;
    onconfirm: () => void;
    oncancel: () => void;
  } = $props();

  let confirmBtn = $state<HTMLButtonElement | undefined>();
  let dialogEl = $state<HTMLDivElement | undefined>();
  let prevFocus: Element | null = null;

  // Save the previously focused element and focus the confirm button when the
  // dialog opens; restore focus when it closes.
  $effect(() => {
    if (open) {
      prevFocus = document.activeElement;
      confirmBtn?.focus();
    } else if (prevFocus instanceof HTMLElement) {
      prevFocus.focus();
      prevFocus = null;
    }
  });

  let body = $derived.by(() => {
    const freed = sizeBytes ? ` This frees about ${humanSize(sizeBytes)} of disk space.` : "";
    return `This removes the application from your system.${freed} You may be asked for your password to continue.`;
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && !busy) {
      oncancel();
      return;
    }

    // Tab trap: keep focus inside the dialog.
    if (e.key === "Tab" && dialogEl) {
      const focusable = Array.from(
        dialogEl.querySelectorAll<HTMLElement>(
          'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
        ),
      ).filter((el) => !el.closest("[disabled]"));

      if (focusable.length === 0) {
        e.preventDefault();
        return;
      }

      const first = focusable[0];
      const last = focusable[focusable.length - 1];

      if (e.shiftKey) {
        if (document.activeElement === first) {
          e.preventDefault();
          last.focus();
        }
      } else {
        if (document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    }
  }

  function handleBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget && !busy) oncancel();
  }
</script>

<div
  class="modal-scrim"
  class:open
  aria-hidden={!open}
  onclick={handleBackdrop}
>
  {#if open}
    <div
      class="modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="cd-title"
      aria-describedby="cd-message"
      tabindex="-1"
      bind:this={dialogEl}
      onkeydown={handleKeydown}
    >
      <div class="modal-icon" aria-hidden="true">
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 9v4"/><path d="M12 17h.01"/><path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/></svg>
      </div>
      <h2 id="cd-title">Uninstall {name}?</h2>
      <p id="cd-message">{body}</p>
      {#if aptNote}
        <div class="apt-note">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/></svg>
          <span>Some dependent packages installed with this app may also be removed.</span>
        </div>
      {/if}
      <div class="modal-actions">
        <button class="btn btn-ghost" disabled={busy} onclick={oncancel}>
          Cancel
        </button>
        <button
          class="btn btn-destructive-solid"
          disabled={busy}
          bind:this={confirmBtn}
          onclick={onconfirm}
        >
          {#if busy}
            <Spinner size={16} />
          {/if}
          Uninstall
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .modal-scrim {
    position: fixed;
    inset: 0;
    background: rgba(20, 20, 19, 0.32);
    backdrop-filter: blur(3px);
    z-index: 200;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    opacity: 0;
    pointer-events: none;
    transition: opacity 180ms var(--ease);
  }
  .modal-scrim.open {
    opacity: 1;
    pointer-events: auto;
  }
  :global([data-theme="dark"]) .modal-scrim {
    background: rgba(0, 0, 0, 0.55);
  }
  .modal {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 14px;
    box-shadow: 0 16px 48px rgba(20, 20, 19, 0.18);
    width: 100%;
    max-width: 440px;
    padding: 28px 28px 20px;
    transform: translateY(8px) scale(0.98);
    transition: transform 180ms var(--ease);
  }
  .modal-scrim.open .modal {
    transform: translateY(0) scale(1);
  }
  :global([data-theme="dark"]) .modal {
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5);
  }
  .modal-icon {
    width: 44px;
    height: 44px;
    border-radius: 12px;
    background: var(--destructive-tint);
    color: var(--destructive);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 16px;
  }
  .modal h2 {
    font-family: var(--serif);
    font-size: 22px;
    font-weight: 500;
    letter-spacing: -0.01em;
    margin: 0 0 10px;
    color: var(--text);
  }
  .modal p {
    margin: 0 0 12px;
    color: var(--text-muted);
    font-size: 14px;
    line-height: 1.55;
  }
  .apt-note {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    background: var(--apt-tint);
    border: 1px solid color-mix(in oklab, var(--apt) 30%, transparent);
    border-radius: 8px;
    padding: 10px 12px;
    font-size: 13px;
    color: var(--text);
    margin: 4px 0 18px;
  }
  .apt-note svg {
    color: var(--apt);
    flex-shrink: 0;
    margin-top: 1px;
  }
  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 8px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 0 16px;
    height: 38px;
    border-radius: var(--radius-ctrl);
    font-size: 13px;
    font-weight: 500;
    transition: all 150ms var(--ease);
    border: 1px solid transparent;
    white-space: nowrap;
  }
  .btn:disabled {
    cursor: not-allowed;
    opacity: 0.7;
  }
  .btn-ghost {
    color: var(--text-muted);
    border-color: var(--border);
    background: var(--surface);
  }
  .btn-ghost:hover:not(:disabled) {
    color: var(--text);
    border-color: var(--border-strong);
    background: var(--surface-2);
  }
  .btn-destructive-solid {
    color: #fff;
    background: var(--destructive);
    border-color: var(--destructive);
  }
  .btn-destructive-solid:hover:not(:disabled) {
    background: var(--destructive-hover);
    border-color: var(--destructive-hover);
  }
</style>
