<script lang="ts">
  import type { App } from "$lib/types";
  import { selected, removeApp, pushToast } from "$lib/stores";
  import { iconSrc, getAppDetails, uninstallApp } from "$lib/api";
  import { humanSize, humanDate } from "$lib/format";
  import SourceBadge from "./SourceBadge.svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import { fly, fade } from "svelte/transition";

  let app = $derived($selected);

  let src = $derived(app ? iconSrc(app) : null);
  let iconFailed = $state(false);

  let descLoading = $state(false);
  let description = $state<string | null>(null);

  let drawerEl: HTMLElement | undefined = $state();
  let prevFocus: HTMLElement | null = null;

  // Uninstall confirm dialog state
  let confirmOpen = $state(false);
  let busy = $state(false);

  $effect(() => {
    if (app) {
      prevFocus = document.activeElement as HTMLElement | null;
      drawerEl?.focus();
    } else if (prevFocus) {
      prevFocus.focus();
      prevFocus = null;
    }
  });

  function close() {
    selected.set(null);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && !confirmOpen) close();
  }

  // Lazy-load the long description whenever a new app is opened.
  $effect(() => {
    const current = app;
    iconFailed = false;
    if (!current) {
      description = null;
      descLoading = false;
      return;
    }

    const uid = current.uid;
    descLoading = true;
    description = null;
    let cancelled = false;

    getAppDetails(uid)
      .then((text) => {
        if (!cancelled) description = text;
      })
      .catch(() => {
        if (!cancelled) description = null;
      })
      .finally(() => {
        if (!cancelled) descLoading = false;
      });

    return () => {
      cancelled = true;
    };
  });

  let initial = $derived(
    app ? app.name.trim().charAt(0).toUpperCase() || "?" : "?",
  );

  type Row = { label: string; value: string; mono?: boolean; truncate?: boolean };
  let rows = $derived.by<Row[]>(() => {
    if (!app) return [];
    return [
      { label: "Size", value: humanSize(app.size_bytes) },
      { label: "Installed", value: humanDate(app.install_date) },
      { label: "Publisher", value: app.publisher ?? "—" },
      { label: "Package", value: app.pkg_ref || "—", mono: true, truncate: true },
      { label: "Command", value: app.exec ?? "—", mono: true, truncate: true },
    ];
  });

  // Build the confirmation message dynamically based on app properties.
  let confirmMessage = $derived.by<string>(() => {
    if (!app) return "";
    let msg = `This removes ${app.name} (${app.source}).`;
    if (app.size_bytes) {
      msg += ` Frees about ${humanSize(app.size_bytes)}.`;
    }
    msg += ` You may be asked for your password.`;
    if (app.source === "apt") {
      msg += ` It may also remove other packages that depend on it.`;
    }
    return msg;
  });

  /** Detect a PermissionDenied error from the Tauri invoke result. */
  function isPermissionDenied(e: unknown): boolean {
    if (typeof e === "object" && e !== null) {
      const obj = e as Record<string, unknown>;
      if (obj["kind"] === "PermissionDenied") return true;
    }
    if (typeof e === "string") {
      return e.includes("PermissionDenied") || e.toLowerCase().includes("permission denied");
    }
    return false;
  }

  async function handleUninstallConfirm() {
    if (!app) return;
    const { uid, name } = app;
    busy = true;
    try {
      await uninstallApp(uid);
      pushToast("success", `${name} uninstalled`);
      // removeApp clears selected (closes the drawer) if this app was open.
      removeApp(uid);
      confirmOpen = false;
    } catch (e: unknown) {
      const msg = isPermissionDenied(e)
        ? "Authentication cancelled — nothing was removed."
        : (typeof e === "object" && e !== null && "message" in e
            ? String((e as { message: unknown }).message)
            : String(e));
      pushToast("error", msg);
      // Keep the drawer open; close the confirm dialog.
      confirmOpen = false;
    } finally {
      busy = false;
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if app}
  <!-- Backdrop: click to dismiss. Keyboard dismissal is handled by Esc above. -->
  <div
    class="backdrop"
    transition:fade={{ duration: 150 }}
    onclick={close}
    aria-hidden="true"
  ></div>

  <div
    class="drawer"
    role="dialog"
    aria-modal="true"
    aria-label="{app.name} details"
    tabindex="-1"
    bind:this={drawerEl}
    transition:fly={{ x: 380, duration: 200 }}
  >
    <header class="head">
      <div class="icon">
        {#if src && !iconFailed}
          <img src={src} alt="" onerror={() => (iconFailed = true)} />
        {:else}
          <span class="fallback" aria-hidden="true">{initial}</span>
        {/if}
      </div>
      <div class="head-text">
        <h2 title={app.name}>{app.name}</h2>
        {#if app.summary}
          <p class="summary">{app.summary}</p>
        {/if}
        <div class="head-meta">
          <SourceBadge source={app.source} />
          {#if app.version}<span class="version">{app.version}</span>{/if}
        </div>
      </div>
      <button type="button" class="close" aria-label="Close details" onclick={close}>
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path
            d="M4.22 4.22a.75.75 0 0 1 1.06 0L8 6.94l2.72-2.72a.75.75 0 1 1 1.06 1.06L9.06 8l2.72 2.72a.75.75 0 1 1-1.06 1.06L8 9.06l-2.72 2.72a.75.75 0 0 1-1.06-1.06L6.94 8 4.22 5.28a.75.75 0 0 1 0-1.06Z"
            fill="currentColor"
          />
        </svg>
      </button>
    </header>

    <div class="content">
      <dl class="meta-rows">
        {#each rows as row (row.label)}
          <div class="row">
            <dt>{row.label}</dt>
            <dd class:mono={row.mono} class:truncate={row.truncate} title={row.truncate ? row.value : undefined}>
              {row.value}
            </dd>
          </div>
        {/each}

        <div class="row">
          <dt>Categories</dt>
          <dd>
            {#if app.categories.length}
              <div class="chips">
                {#each app.categories as c (c)}
                  <span class="chip">{c}</span>
                {/each}
              </div>
            {:else}
              —
            {/if}
          </dd>
        </div>
      </dl>

      <section class="description">
        <h3>Description</h3>
        {#if descLoading}
          <p class="muted">Loading description…</p>
        {:else if description}
          <p class="desc-text">{description}</p>
        {:else}
          <p class="muted">No description available.</p>
        {/if}
      </section>
    </div>

    <footer class="foot">
      <button
        type="button"
        class="uninstall"
        disabled={!app.removable || busy}
        title={app.removable ? undefined : (app.protected_reason ?? "This package is protected")}
        onclick={() => { confirmOpen = true; }}
      >
        Uninstall
      </button>
    </footer>
  </div>

  <ConfirmDialog
    open={confirmOpen}
    title="Uninstall {app.name}?"
    message={confirmMessage}
    confirmLabel="Uninstall"
    destructive={true}
    {busy}
    onconfirm={handleUninstallConfirm}
    oncancel={() => { if (!busy) confirmOpen = false; }}
  />
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
    background: rgba(0, 0, 0, 0.32);
  }

  .drawer {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    z-index: 41;
    width: min(380px, 100%);
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border-left: 1px solid var(--border);
    box-shadow: var(--shadow-lg);
  }

  .head {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 18px;
    border-bottom: 1px solid var(--border);
  }

  .icon {
    flex: none;
    width: 64px;
    height: 64px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .icon img {
    width: 64px;
    height: 64px;
    object-fit: contain;
  }
  .fallback {
    width: 64px;
    height: 64px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 16px;
    background: var(--surface-hover);
    border: 1px solid var(--border);
    color: var(--muted);
    font-size: 30px;
    font-weight: 600;
  }

  .head-text {
    flex: 1;
    min-width: 0;
  }
  .head-text h2 {
    margin: 0;
    font-size: 18px;
    font-weight: 700;
    line-height: 1.25;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .summary {
    margin: 4px 0 0;
    font-size: 13px;
    color: var(--muted);
  }
  .head-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
  }
  .head-meta .version {
    font-size: 12px;
    color: var(--muted);
  }

  .close {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border: none;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }
  .close:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .close svg {
    width: 15px;
    height: 15px;
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: 16px 18px;
  }

  .meta-rows {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0;
  }
  .row {
    display: grid;
    grid-template-columns: 92px 1fr;
    gap: 12px;
    padding: 9px 0;
    border-bottom: 1px solid var(--border);
    align-items: start;
  }
  .row:last-child {
    border-bottom: none;
  }
  dt {
    font-size: 12px;
    color: var(--muted);
    font-weight: 600;
  }
  dd {
    margin: 0;
    font-size: 13px;
    min-width: 0;
  }
  dd.mono {
    font-family: var(--font-mono);
    font-size: 12px;
  }
  dd.truncate {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    padding: 2px 8px;
    font-size: 11px;
    border-radius: var(--radius-pill);
    background: var(--surface-hover);
    border: 1px solid var(--border);
    color: var(--muted);
  }

  .description {
    margin-top: 18px;
  }
  .description h3 {
    margin: 0 0 6px;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  .desc-text {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
    white-space: pre-wrap;
  }
  .muted {
    margin: 0;
    color: var(--muted);
    font-size: 13px;
  }

  .foot {
    padding: 14px 18px;
    border-top: 1px solid var(--border);
  }
  .uninstall {
    width: 100%;
    padding: 9px 14px;
    border: 1px solid var(--destructive);
    border-radius: var(--radius-ctrl);
    background: var(--surface);
    color: var(--destructive);
    font: inherit;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
  }
  .uninstall:hover:not(:disabled) {
    background: var(--destructive);
    color: #fff;
  }
  .uninstall:disabled {
    border-color: var(--border);
    color: var(--muted);
    cursor: not-allowed;
    opacity: 0.55;
  }
</style>
