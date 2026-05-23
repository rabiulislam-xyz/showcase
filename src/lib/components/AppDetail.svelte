<script lang="ts">
  import { selected, removeApp, pushToast } from "$lib/stores";
  import { iconSrc, getAppDetails, uninstallApp } from "$lib/api";
  import { humanSize, humanDate } from "$lib/format";
  import { tileColor, tileInitial, sourceLabel } from "$lib/avatar";
  import ConfirmDialog from "./ConfirmDialog.svelte";

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
      // Restore on the next frame: the opener lives in the background container
      // that was inert while the drawer was open, and an inert element can't take
      // focus until the attribute is gone from the DOM.
      const target = prevFocus;
      prevFocus = null;
      requestAnimationFrame(() => target.focus());
    }
  });

  function close() {
    selected.set(null);
  }

  function onKeydown(e: KeyboardEvent) {
    if (!app) return;

    // Esc closes the drawer, but not while the confirm dialog owns the layer.
    if (e.key === "Escape" && !confirmOpen) {
      close();
      return;
    }

    // While the confirm dialog is open it runs its own trap; don't double-handle.
    if (e.key === "Tab" && !confirmOpen && drawerEl) {
      trapTab(e);
    }
  }

  /** Keep Tab/Shift+Tab focus cycling within the open drawer. */
  function trapTab(e: KeyboardEvent) {
    if (!drawerEl) return;
    const focusable = Array.from(
      drawerEl.querySelectorAll<HTMLElement>(
        'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
      ),
    ).filter((el) => !el.closest("[disabled]"));

    if (focusable.length === 0) {
      e.preventDefault();
      drawerEl.focus();
      return;
    }

    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    const active = document.activeElement;

    // Wrap at the edges; also pull focus back in if it has escaped the drawer.
    if (e.shiftKey) {
      if (active === first || !drawerEl.contains(active)) {
        e.preventDefault();
        last.focus();
      }
    } else if (active === last || !drawerEl.contains(active)) {
      e.preventDefault();
      first.focus();
    }
  }

  // Lazy-load the long description whenever a *different* app is opened.
  // Track only the uid so re-renders that keep the same identity don't refetch.
  $effect(() => {
    const uid = app?.uid;
    iconFailed = false;
    if (!uid) {
      description = null;
      descLoading = false;
      return;
    }

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

  let initial = $derived(app ? tileInitial(app.name) : "?");
  let tint = $derived(app ? tileColor(app.name) : "var(--accent)");

  type Row = { label: string; value: string; mono?: boolean };
  let rows = $derived.by<Row[]>(() => {
    if (!app) return [];
    return [
      { label: "Version", value: app.version ?? "—" },
      { label: "Size", value: humanSize(app.size_bytes) },
      { label: "Installed", value: humanDate(app.install_date) },
      { label: "Publisher", value: app.publisher ?? "—" },
      { label: "Package id", value: app.pkg_ref || "—", mono: true },
      { label: "Command", value: app.exec ?? "—", mono: true },
    ];
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
        : typeof e === "object" && e !== null && "message" in e
          ? String((e as { message: unknown }).message)
          : String(e);
      pushToast("error", msg);
      // Keep the drawer open; close the confirm dialog.
      confirmOpen = false;
    } finally {
      busy = false;
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<!-- Scrim and drawer stay mounted so open/close transitions via CSS. -->
<div
  class="drawer-scrim"
  class:open={!!app}
  onclick={close}
  aria-hidden="true"
></div>

<div
  class="drawer"
  class:open={!!app}
  role="dialog"
  aria-modal="true"
  aria-label={app ? `${app.name} details` : "App details"}
  aria-hidden={!app}
  tabindex="-1"
  bind:this={drawerEl}
>
  {#if app}
    <div class="drawer-head">
      {#if src && !iconFailed}
        <img class="drawer-icon-img" src={src} alt="" onerror={() => (iconFailed = true)} />
      {:else}
        <div class="drawer-icon" style="background:{tint}" aria-hidden="true">{initial}</div>
      {/if}
      <div class="drawer-title-block">
        <h2 class="drawer-title" title={app.name}>{app.name}</h2>
        <span class="tag {app.source}">{sourceLabel(app.source)}</span>
      </div>
      <button class="drawer-close" aria-label="Close details" onclick={close}>
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
      </button>
    </div>

    <div class="drawer-body">
      <h4 class="drawer-section-title">Details</h4>
      <div class="meta-list">
        {#each rows as row (row.label)}
          <div class="meta-key">{row.label}</div>
          <div class="meta-val" class:mono={row.mono}>{row.value}</div>
        {/each}
        <div class="meta-key">Categories</div>
        <div class="meta-val">
          {#if app.categories.length}
            <div class="chips">
              {#each app.categories as c (c)}
                <span class="chip">{c}</span>
              {/each}
            </div>
          {:else}
            —
          {/if}
        </div>
      </div>

      <h4 class="drawer-section-title">About</h4>
      {#if descLoading}
        <p class="description muted">Loading description…</p>
      {:else if description}
        <p class="description">{description}</p>
      {:else}
        <p class="description muted">No description available.</p>
      {/if}
    </div>

    <div class="drawer-foot">
      <span class="footnote">Installed via {sourceLabel(app.source)}</span>
      <button
        class="btn btn-destructive"
        disabled={!app.removable || busy}
        title={app.removable ? undefined : (app.protected_reason ?? "This package is protected")}
        onclick={() => { confirmOpen = true; }}
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 6h18"/><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/></svg>
        Uninstall
      </button>
    </div>

    <ConfirmDialog
      open={confirmOpen}
      name={app.name}
      sizeBytes={app.size_bytes}
      aptNote={app.source === "apt"}
      {busy}
      onconfirm={handleUninstallConfirm}
      oncancel={() => { if (!busy) confirmOpen = false; }}
    />
  {/if}
</div>

<style>
  .drawer-scrim {
    position: fixed;
    inset: 0;
    background: rgba(20, 20, 19, 0.2);
    backdrop-filter: blur(2px);
    z-index: 100;
    opacity: 0;
    pointer-events: none;
    transition: opacity 200ms var(--ease);
  }
  .drawer-scrim.open {
    opacity: 1;
    pointer-events: auto;
  }
  :global([data-theme="dark"]) .drawer-scrim {
    background: rgba(0, 0, 0, 0.45);
  }

  .drawer {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    width: min(480px, 92vw);
    background: var(--surface);
    border-left: 1px solid var(--border);
    box-shadow: var(--shadow-drawer);
    z-index: 101;
    transform: translateX(100%);
    transition: transform 250ms var(--ease);
    display: flex;
    flex-direction: column;
  }
  .drawer.open {
    transform: translateX(0);
  }

  .drawer-head {
    padding: 24px 28px 20px;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: flex-start;
    gap: 16px;
  }
  .drawer-icon,
  .drawer-icon-img {
    width: 64px;
    height: 64px;
    border-radius: 16px;
    flex-shrink: 0;
  }
  .drawer-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--serif);
    font-size: 30px;
    font-weight: 500;
    color: #fff;
  }
  .drawer-icon-img {
    object-fit: contain;
  }
  .drawer-title-block {
    flex: 1;
    min-width: 0;
    padding-top: 4px;
  }
  .drawer-title {
    font-family: var(--serif);
    font-size: 24px;
    font-weight: 500;
    letter-spacing: -0.015em;
    margin: 0 0 8px;
    line-height: 1.2;
    color: var(--text);
    word-break: break-word;
  }
  .drawer-close {
    width: 32px;
    height: 32px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    transition: all 150ms var(--ease);
    margin-left: auto;
    flex-shrink: 0;
  }
  .drawer-close:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .drawer-body {
    flex: 1;
    overflow-y: auto;
    padding: 24px 28px;
  }

  .meta-list {
    display: grid;
    grid-template-columns: 110px 1fr;
    gap: 14px 16px;
    margin: 0 0 24px;
  }
  .meta-key {
    color: var(--text-muted);
    font-size: 13px;
    padding-top: 2px;
  }
  .meta-val {
    color: var(--text);
    font-size: 13px;
    word-break: break-word;
    min-width: 0;
  }
  .meta-val.mono {
    font-family: var(--mono);
    font-size: 12px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    font-size: 11px;
    padding: 3px 8px;
    border-radius: 5px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text-muted);
    font-weight: 500;
  }
  .drawer-section-title {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-faint);
    font-weight: 600;
    margin: 0 0 10px;
  }
  .description {
    color: var(--text);
    font-size: 14px;
    line-height: 1.65;
    margin: 0;
    text-wrap: pretty;
    white-space: pre-wrap;
  }
  .description.muted {
    color: var(--text-muted);
  }

  .drawer-foot {
    padding: 18px 28px;
    border-top: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    background: var(--surface);
  }
  .footnote {
    font-size: 12px;
    color: var(--text-faint);
  }

  .tag {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    font-weight: 500;
    padding: 3px 8px;
    border-radius: 5px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .tag.apt {
    color: var(--apt);
    background: var(--apt-tint);
  }
  .tag.flatpak {
    color: var(--flatpak);
    background: var(--flatpak-tint);
  }
  .tag.snap {
    color: var(--snap);
    background: var(--snap-tint);
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
  .btn-destructive {
    color: var(--destructive);
    border-color: var(--border);
    background: var(--surface);
  }
  .btn-destructive:hover:not(:disabled) {
    color: #fff;
    background: var(--destructive);
    border-color: var(--destructive);
  }
  .btn-destructive:disabled {
    color: var(--text-faint);
    cursor: not-allowed;
    opacity: 0.7;
  }
</style>
