<!-- V1051-01 A dropdown in the app's look, for a short list of choices. A button that opens a
     listbox under it (it flips and stays in the window). Focus stays on the button: arrows,
     Home/End, typing a name, Enter or Space pick, Escape closes (lib/escape.ts). Same size as `.field`. -->
<script lang="ts">
  import { layer } from '../lib/escape';
  import { floating } from '../lib/place';
  import Icon from './Icon.svelte';
  import { type Choice, matchTyped, moveFor } from './select.ts';

  interface Props {
    options: readonly Choice[];
    value: string;
    label: string;
    onchange: (value: string) => void;
    /** Called when the list opens, e.g. to refresh it. */
    onopen?: () => void;
    width?: string;
  }

  let { options, value, label, onchange, onopen, width = '300px' }: Props = $props();

  const uid = $props.id();
  let open = $state(false);
  let active = $state(0);
  let button = $state<HTMLButtonElement>();
  let list = $state<HTMLElement>();
  let typed = '';
  let typedAt = 0;

  const selected = $derived(options.findIndex((o) => o.value === value));
  const shown = $derived(options[selected]?.label ?? '');

  function show() {
    if (open || !options.length) return;
    onopen?.();
    active = Math.max(selected, 0);
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

  // Keeps the active choice in view in a long list.
  $effect(() => {
    if (open) list?.querySelector('[data-active="true"]')?.scrollIntoView({ block: 'nearest' });
  });

  function onkey(e: KeyboardEvent) {
    if (e.ctrlKey || e.altKey || e.metaKey) return;
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

<span class="select" style:width style:--w={width}>
  <button
    bind:this={button}
    type="button"
    class="field trigger"
    role="combobox"
    aria-label={label}
    aria-haspopup="listbox"
    title={shown}
    aria-expanded={open}
    aria-controls="{uid}-list"
    aria-activedescendant={open ? `${uid}-${active}` : undefined}
    onclick={() => (open ? close() : show())}
    onkeydown={onkey}
  >
    <span class="text">{shown}</span>
    <Icon name="chevronDown" size={14} />
  </button>
  {#if open}
    <div id="{uid}-list" class="list" role="listbox" aria-label={label} bind:this={list} use:floating={{ anchor: button, side: 'below', align: 'start' }}>
      {#each options as o, i (o.value)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          id="{uid}-{i}"
          class="option"
          role="option"
          tabindex="-1"
          aria-selected={i === selected}
          data-active={i === active}
          class:active={i === active}
          class:on={i === selected}
          onmousemove={() => (active = i)}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => pick(i)}
        >
          {o.label}
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
  .trigger :global(svg) {
    flex: none;
    color: var(--dim);
  }
  .list {
    position: fixed;
    z-index: var(--z-popover);
    min-width: var(--w);
    max-width: min(480px, calc(100vw - 16px));
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
    min-height: 28px;
    padding: 0 8px;
    border-radius: 2px;
    color: var(--text);
    cursor: pointer;
    overflow-wrap: anywhere;
  }
  .option.active {
    background: var(--row-hover);
    box-shadow: inset 0 0 0 1px var(--focus-ring);
  }
  .option.on {
    background: var(--accent-dim);
    color: var(--accent-text);
  }
</style>
