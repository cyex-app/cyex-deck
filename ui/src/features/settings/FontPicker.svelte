<!-- The chat font: a fixed list of 8 (design/fonts.ts), each name in its own face. Arrows move, Enter picks. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import Popover from '../../components/Popover.svelte';
  import { FONTS, fontName, fontValue } from '../../design/fonts';

  interface Props {
    /** Empty = the app font (IBM Plex Sans). */
    value: string;
    onpick: (value: string) => void;
  }

  let { value, onpick }: Props = $props();

  let open = $state(false);

  const current = $derived(fontName(value));
  const face = (name: string) => `"${name}"`;

  function pick(name: string) {
    onpick(fontValue(name));
    open = false;
  }
</script>

<div class="wrap">
  <button class="field trigger" aria-haspopup="menu" aria-expanded={open} onclick={() => (open = !open)}>
    <span class="name" style:font-family={face(current)}>{current}</span>
    <Icon name="chevronDown" size={14} />
  </button>
  <Popover bind:open align="end" role="menu" label="Chat font">
    <div class="panel">
      {#each FONTS as name (name)}
        <button class="row" role="menuitemradio" aria-checked={name === current} onclick={() => pick(name)}>
          <span class="face" style:font-family={face(name)}>{name}</span>
          {#if name === current}<span class="tick"><Icon name="check" size={14} /></span>{/if}
        </button>
      {/each}
    </div>
  </Popover>
</div>

<style>
  .wrap {
    position: relative;
  }
  .trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 200px;
    height: 30px;
    padding: 0 10px;
    text-align: left;
  }
  .trigger :global(svg) {
    flex: none;
    color: var(--dim);
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .panel {
    display: flex;
    flex-direction: column;
    min-width: 200px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    height: 30px;
    padding: 0 8px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--text);
    font-size: 0.95rem;
    text-align: left;
  }
  .row:hover {
    background: var(--surface-hover);
  }
  /* V108-F55 The menus' focus: a ring inside the row, never the hover tint. */
  .row:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: -2px;
  }
  .tick {
    display: flex;
    flex: none;
    color: var(--accent-text);
  }
  /* Not `.label`: that global class (tokens.css) is the small capitals of section titles. */
  .face {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
