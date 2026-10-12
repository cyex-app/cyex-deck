<!-- V102-20 The flag on a greyed-out tool: which sign-in it needs. A click goes there;
     `still` draws it without a button, for flags inside another button or menu item. -->
<script lang="ts">
  import type { TokenKind } from '../ipc/bindings/TokenKind';
  import { FLAGS } from '../lib/gate';
  import { gate } from '../lib/gate.svelte';
  import Icon from './Icon.svelte';
  import { tooltip } from './tooltip.svelte';

  let { kind, still = false }: { kind: TokenKind; still?: boolean } = $props();

  const flag = $derived(FLAGS[kind]);

  function go(e: MouseEvent) {
    e.stopPropagation();
    gate.ask(kind);
  }
</script>

{#if still}
  <span class="flag"><Icon name="lock" size={11} />{flag.label}</span>
{:else}
  <button class="flag" onclick={go} use:tooltip={{ label: flag.label, hint: flag.hint }}>
    <Icon name="lock" size={11} />{flag.label}
  </button>
{/if}

<style>
  .flag {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 4px;
    height: 18px;
    padding: 0 6px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.7rem;
    font-weight: 600;
    line-height: 1;
    white-space: nowrap;
  }
  button.flag:hover {
    border-color: var(--accent-border);
    color: var(--text);
  }
</style>
