<script lang="ts">
  interface Option {
    value: string;
    label: string;
  }

  interface Props {
    value: string;
    options: Option[];
    onSelect: (value: string) => void;
    ariaLabel: string;
    buttonLabel?: string;
  }

  let { value, options, onSelect, ariaLabel, buttonLabel }: Props = $props();

  // Unique per-instance id base so multiple dropdowns on one page never collide.
  const uid = $props.id();
  const listboxId = `${uid}-listbox`;
  const optionId = (i: number) => `${uid}-opt-${i}`;

  let selectedLabel = $derived(
    options.find((o) => o.value === value)?.label ?? buttonLabel ?? "",
  );

  let open = $state(false);
  let wrap = $state<HTMLDivElement | undefined>();
  let btn = $state<HTMLButtonElement | undefined>();
  let listEl = $state<HTMLDivElement | undefined>();
  // Index highlighted via aria-activedescendant while the listbox is open.
  let activeIndex = $state(0);

  let activeOptionId = $derived(open ? optionId(activeIndex) : undefined);

  function openMenu() {
    // Start the highlight on the currently selected option.
    const i = options.findIndex((o) => o.value === value);
    activeIndex = i >= 0 ? i : 0;
    open = true;
    // Move focus into the list once it renders.
    requestAnimationFrame(() => listEl?.focus());
  }

  function closeMenu(returnFocus = true) {
    open = false;
    if (returnFocus) btn?.focus();
  }

  function toggle() {
    if (open) closeMenu();
    else openMenu();
  }

  function choose(v: string) {
    onSelect(v);
    closeMenu();
  }

  function onListKeydown(e: KeyboardEvent) {
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        activeIndex = (activeIndex + 1) % options.length;
        break;
      case "ArrowUp":
        e.preventDefault();
        activeIndex = (activeIndex - 1 + options.length) % options.length;
        break;
      case "Home":
        e.preventDefault();
        activeIndex = 0;
        break;
      case "End":
        e.preventDefault();
        activeIndex = options.length - 1;
        break;
      case "Enter":
      case " ":
        e.preventDefault();
        choose(options[activeIndex].value);
        break;
      case "Escape":
        e.preventDefault();
        closeMenu();
        break;
      case "Tab":
        // Let focus leave naturally, but close the popup behind it.
        closeMenu(false);
        break;
    }
  }

  function onDocClick(e: MouseEvent) {
    if (open && wrap && !wrap.contains(e.target as Node)) {
      closeMenu(false);
    }
  }
</script>

<svelte:window onclick={onDocClick} />

<div class="dd-wrap" bind:this={wrap}>
  <button
    class="dd-select"
    bind:this={btn}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-controls={listboxId}
    aria-label={ariaLabel}
    onclick={toggle}
  >
    <span>{selectedLabel}</span>
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg>
  </button>
  {#if open}
    <div
      id={listboxId}
      class="dd-menu open"
      role="listbox"
      tabindex="-1"
      aria-label={ariaLabel}
      aria-activedescendant={activeOptionId}
      bind:this={listEl}
      onkeydown={onListKeydown}
    >
      {#each options as o, i (o.value)}
        <!-- Keyboard is handled on the listbox via aria-activedescendant;
             the click handler is only a pointer affordance. -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          id={optionId(i)}
          class="dd-menu-item"
          class:active={i === activeIndex}
          role="option"
          tabindex="-1"
          aria-selected={value === o.value}
          onclick={() => choose(o.value)}
          onmousemove={() => (activeIndex = i)}
        >
          <span>{o.label}</span>
          <svg class="check" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 6 9 17l-5-5"/></svg>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .dd-wrap {
    position: relative;
  }
  .dd-select {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px 6px 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-ctrl);
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
    transition: all 150ms var(--ease);
  }
  .dd-select:hover {
    border-color: var(--border-strong);
  }
  .dd-menu {
    position: absolute;
    right: 0;
    margin-top: 4px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-ctrl);
    box-shadow: var(--shadow);
    min-width: 200px;
    padding: 4px;
    z-index: 100;
  }
  /* The listbox container takes focus (aria-activedescendant model); the
     highlighted option is shown via .active, so suppress the container ring. */
  .dd-menu:focus-visible {
    outline: none;
    box-shadow: var(--shadow);
  }
  .dd-menu-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    font-size: 13px;
    color: var(--text);
    text-align: left;
    cursor: pointer;
    transition: background 150ms var(--ease);
  }
  .dd-menu-item.active {
    background: var(--surface-2);
  }
  .dd-menu-item .check {
    color: var(--accent);
    opacity: 0;
  }
  .dd-menu-item[aria-selected="true"] .check {
    opacity: 1;
  }
</style>
