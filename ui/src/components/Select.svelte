<!-- V1051-01 A dropdown in the app's look, for a short list of choices. A button that opens a
     listbox under it (it flips and stays in the window). Focus stays on the button: arrows,
     Home/End, typing a name, Enter or Space pick, Escape closes (lib/escape.ts). Same size as `.field`. -->
<script lang="ts">
  import { layer } from '../lib/escape';
  import { byKeyboard } from '../lib/motion';
  import { floating } from '../lib/place';
  import Avatar from './Avatar.svelte';
  import Icon from './Icon.svelte';
  import { type Choice, matchTyped, moveFor } from './select.ts';
  import { tooltip } from './tooltip.svelte';
  import { scrollWithin } from '../lib/scrollWithin';

  interface Props {
    options: readonly Choice[];
    value: string;
    label: string;
    onchange: (value: string) => void;
    /** Called when the list opens, e.g. to refresh it. */
    onopen?: () => void;
    width?: string;
    disabled?: boolean;
  }

  let { options, value, label, onchange, onopen, width = '300px', disabled = false }: Props = $props();

  const uid = $props.id();
  let open = $state(false);
  let active = $state(0);
  let button = $state<HTMLButtonElement>();
  let list = $state<HTMLElement>();
  let text = $state<HTMLElement>();
  /** The trigger's width in px when the list opened; `width` may be a percentage. */
  let triggerW = $state(0);
  /** V108-F55 The list opened from the keyboard: no scale-in. The active row shows the focus
   *  ring only while the keyboard moved it; under the pointer it's the hover tint. */
  let keyed = $state(false);
  let byKey = $state(false);
  let typed = '';
  let typedAt = 0;

  const selected = $derived(options.findIndex((o) => o.value === value));
  const shown = $derived(options[selected]?.label ?? '');
  const shownPicture = $derived(options[selected]?.picture);

  function show() {
    if (open || disabled || !options.length) return;
    onopen?.();
    active = Math.max(selected, 0);
    triggerW = button?.getBoundingClientRect().width ?? 0;
    keyed = byKey = byKeyboard();
    open = true;
  }

  function close() {
    open = false;
  }

  function pick(i: number) {
    const o = options[i];
    close();
    if (o && o.value !== value) onchange(o.value);
  }

  $effect(() => {
    if (!open) return;
    const off = layer('select', close);
    const onDown = (e: MouseEvent) => {
      const t = e.target as Node;
      if (!list?.contains(t) && !button?.contains(t)) close();
    };
    const onBlur = () => close();
    addEventListener('mousedown', onDown, true);
    addEventListener('blur', onBlur);
    return () => {
      off();
      removeEventListener('mousedown', onDown, true);
      removeEventListener('blur', onBlur);
    };
  });

  // V108-F57g The options changed under an open list: close if none are left, else keep the active row inside.
  $effect(() => {
    const n = options.length;
    if (!open) return;
    if (!n) close();
    else if (active > n - 1) active = n - 1;
  });

  // Keeps the active choice in view in a long list.
  $effect(() => {
    void active;
    if (!open) return;
    const row = list?.querySelector<HTMLElement>('[data-active="true"]');
    if (row) scrollWithin(row, 'nearest');
  });

  function onkey(e: KeyboardEvent) {
    if (e.ctrlKey || e.altKey || e.metaKey) return;
    byKey = true;
    if (e.key === 'Tab') return close();
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      // A space in the middle of a typed name belongs to the name.
      if (e.key === ' ' && open && typed && Date.now() - typedAt < 700) return type(' ');
      return open ? pick(active) : show();
    }
    const to = moveFor(e.key, open ? active : selected, options.length);
    if (to !== null) {
      e.preventDefault();
      if (!open) {
        show();
        // Arrows open on the current choice; Home and End on their end.
        if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') active = to;
        return;
      }
      active = to;
      return;
    }
    if (e.key.length === 1) {
      e.preventDefault();
      type(e.key);
    }
  }

  function type(ch: string) {
    const now = Date.now();
    typed = now - typedAt < 700 ? typed + ch : ch;
    typedAt = now;
    const hit = matchTyped(options, typed, open ? active : selected);
    if (hit < 0) return;
    if (open) active = hit;
    else pick(hit);
  }
</script>

<span class="select" style:width>
  <button
    bind:this={button}
    type="button"
    class="field trigger"
    role="combobox"
    aria-label={label}
    {disabled}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-controls={open ? `${uid}-list` : undefined}
    aria-activedescendant={open ? `${uid}-${active}` : undefined}
    onclick={() => (open ? close() : show())}
    onkeydown={onkey}
    use:tooltip={{ label: shown, when: () => !open && !!text && text.scrollWidth > text.clientWidth }}
  >
    {#if shownPicture !== undefined}<span class="pic"><Avatar src={shownPicture} name={shown} size={20} /></span>{/if}
    <span class="text" bind:this={text}>{shown}</span>
    <Icon name="chevronDown" size={14} />
  </button>
  {#if open}
    <div id="{uid}-list" class="list" class:float-in={!keyed} role="listbox" aria-label={label} bind:this={list} style:min-width="{triggerW}px" use:floating={{ anchor: button, side: 'below', align: 'start' }}>
      {#each options as o, i (i)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          id="{uid}-{i}"
          class="option"
          role="option"
          tabindex="-1"
          aria-selected={i === selected}
          data-active={i === active}
          class:active={i === active}
          class:key={byKey}
          class:on={i === selected}
          onmousemove={() => ((active = i), (byKey = false))}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => pick(i)}
        >
          {#if o.picture !== undefined}<span class="pic"><Avatar src={o.picture} name={o.label} size={20} /></span>{/if}
          <span class="choice">{o.label}</span>
          <!-- V108-F55 The pick has a check, not only the tint. -->
          {#if i === selected}<span class="check"><Icon name="check" size={14} /></span>{/if}
        </div>
      {/each}
    </div>
  {/if}
</span>

<style>
  .select {
    display: inline-block;
    max-width: 100%;
  }
  .trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    min-height: 30px;
    text-align: left;
    cursor: pointer;
  }
  .pic {
    display: flex;
    flex: none;
  }
  .trigger .text {
    flex: 1;
  }
  .trigger:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .trigger:focus-visible {
    border-color: var(--accent-border);
    outline: 2px solid var(--focus-ring);
    outline-offset: 1px;
  }
  .text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .trigger > :global(svg) {
    flex: none;
    color: var(--dim);
  }
  .list {
    position: fixed;
    z-index: var(--z-popover);
    max-width: min(480px, calc(100vw - 16px));
    /* V108-F55 A long list scrolls instead of filling the window. */
    max-height: min(360px, calc(100vh - 16px));
    overflow-y: auto;
    padding: 4px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-pop);
  }
  .option {
    display: flex;
    align-items: center;
    /* V108-F55 The shared menu's rows: 30 px, the same corners. */
    min-height: 30px;
    padding: 0 8px;
    border-radius: var(--radius);
    color: var(--text);
    cursor: pointer;
    gap: 8px;
    overflow-wrap: anywhere;
  }
  /* Not .label: that global class is the section heading style. */
  .choice {
    flex: 1;
    min-width: 0;
  }
  .check {
    display: flex;
    flex: none;
  }
  .option.on {
    background: var(--accent-dim);
    color: var(--accent-text);
  }
  .option.active:not(.on) {
    background: var(--surface-hover);
  }
  /* Moved there with the keys: the menus' focus ring, so it never reads as the pointer's row. */
  .option.active.key {
    outline: 2px solid var(--focus-ring);
    outline-offset: -2px;
  }
</style>
