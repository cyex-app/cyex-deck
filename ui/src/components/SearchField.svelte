<!-- A search box: magnifier, the text, and a clear × once there's text. Escape clears the
     text first; on an empty box it leaves the box (or closes the popup it sits in, through
     the Escape layer stack). Other keys (arrows, Enter) go to `onkeydown` for result lists. -->
<script lang="ts">
  import Icon from './Icon.svelte';
  import { tooltip } from './tooltip.svelte';

  interface Props {
    value: string;
    placeholder?: string;
    /** What screen readers call it; the placeholder when left out. */
    label?: string;
    width?: string;
    /** 28 px tall, for toolbars; 30 px (`.field`) otherwise. */
    small?: boolean;
    /** Takes focus when it appears, e.g. at the top of a picker. */
    autofocus?: boolean;
    oninput?: (value: string) => void;
    onkeydown?: (e: KeyboardEvent) => void;
    /** After the text was cleared, by × or Escape. */
    onclear?: () => void;
  }

  let {
    value = $bindable(),
    placeholder = 'Search',
    label,
    width = '100%',
    small = false,
    autofocus = false,
    oninput,
    onkeydown,
    onclear,
  }: Props = $props();

  let input = $state<HTMLInputElement>();

  export function focus() {
    input?.focus();
  }

  $effect(() => {
    if (autofocus) input?.focus();
  });

  function clear() {
    value = '';
    oninput?.('');
    onclear?.();
  }

  function keydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && !e.isComposing) {
      if (value) {
        e.preventDefault();
        e.stopPropagation();
        clear();
        return;
      }
      if (!e.defaultPrevented) input?.blur();
    }
    onkeydown?.(e);
  }
</script>

<label class="search field" class:small style:width>
  <Icon name="search" size={14} />
  <input
    bind:this={input}
    type="text"
    role="searchbox"
    aria-label={label ?? placeholder}
    {placeholder}
    spellcheck="false"
    autocomplete="off"
    bind:value
    data-own-escape={value ? '' : undefined}
    oninput={() => oninput?.(value)}
    onkeydown={keydown}
  />
  {#if value}
    <button
      type="button"
      class="clear"
      aria-label="Clear"
      use:tooltip={'Clear'}
      onmousedown={(e) => e.preventDefault()}
      onclick={() => {
        clear();
        input?.focus();
      }}
    >
      <Icon name="x" size={12} />
    </button>
  {/if}
</label>

<style>
  .search {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    max-width: 100%;
    padding: 0 4px 0 9px;
    color: var(--dim);
    cursor: text;
  }
  .search:focus-within {
    border-color: var(--accent-border);
  }
  .small {
    height: 28px;
  }
  .search > :global(svg) {
    flex: none;
  }
  input {
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 0;
    background: none;
    border: 0;
    /* The box's border shows focus. */
    outline: none;
    color: var(--text);
    font-size: 0.85rem;
  }
  input::placeholder {
    color: var(--dim);
  }
  .clear {
    display: grid;
    place-items: center;
    flex: none;
    width: 20px;
    height: 20px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius-sm);
    color: var(--dim);
    transition:
      background var(--fast),
      color var(--fast);
  }
  .clear:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
</style>
