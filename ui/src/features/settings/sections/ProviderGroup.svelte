<!-- V110-E13 One provider on the Emotes and badges page: a header with a switch for all of its
     settings, a short summary while closed, and its own switches when open. Open or closed is
     remembered; the group's switch is on while any of them is. -->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { onMount } from 'svelte';
  import Icon from '../../../components/Icon.svelte';
  import Row from '../../../components/Row.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import { settings } from '../../../lib/settings.svelte';
  import {
    OPEN_EVENT,
    isOpen,
    masterOn,
    masterStates,
    memo,
    patchFor,
    read,
    setMemo,
    setOpen,
    snapshot,
    summary,
    type Child,
    type Group,
  } from '../emoteGroups';

  let { group, extra }: { group: Group; extra?: Snippet } = $props();

  let open = $state(false);
  const v = $derived(settings.v);
  const on = $derived(masterOn(v, group));
  const single = $derived(group.children.length === 1);

  onMount(() => {
    open = isOpen(group.key);
    const pick = (e: Event) => {
      if ((e as CustomEvent<string>).detail === group.key) open = true;
    };
    window.addEventListener(OPEN_EVENT, pick);
    return () => window.removeEventListener(OPEN_EVENT, pick);
  });

  function toggle() {
    open = !open;
    setOpen(group.key, open);
  }

  async function apply(states: [Child, boolean][]) {
    const p = patchFor(states);
    if (p.chat) await settings.patch('chat', p.chat);
    if (p.emotes) await settings.patch('emotes', p.emotes);
  }

  /** All off remembers what was on; on brings that back. */
  function master(next: boolean) {
    if (!next) setMemo(group.key, snapshot(v, group));
    void apply(masterStates(group, next, next ? memo(group.key) : undefined));
  }
</script>

<div class="group">
  <div class="head" class:single data-setting={single || !open ? `${group.children.map((c) => c.id).join(' ')}` : undefined}>
    {#if single}
      <span class="name single-name">{group.label}</span>
      <span class="dim">{group.children[0].label}</span>
    {:else}
      <button class="fold" aria-expanded={open} onclick={toggle}>
        <Icon name={open ? 'chevronDown' : 'chevronRight'} size={14} />
        <span class="name">{group.label}</span>
        {#if !open}<span class="dim">{summary(v, group)}</span>{/if}
      </button>
    {/if}
    <Toggle label="{group.label}: everything" checked={on} onchange={master} />
  </div>
  {#if !single && open}
    <div class="kids">
      {#each group.children as c (c.id)}
        <Row setting={`${c.id}`} label={c.label} hint={c.hint}>
          <Toggle
            label="{group.label} {c.label.toLowerCase()}"
            checked={read(v, c)}
            onchange={(next) => void apply([[c, next]])}
          />
        </Row>
      {/each}
      {@render extra?.()}
    </div>
  {/if}
</div>

<style>
  .group {
    border-bottom: 1px solid var(--border);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 6px 0;
  }
  .fold {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding: 6px 4px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--text);
    font: inherit;
    text-align: left;
  }
  .fold:hover {
    background: var(--row-hover);
  }
  .fold :global(svg) {
    flex: none;
    color: var(--dim);
  }
  .name {
    font-weight: 600;
  }
  /* Lines up with the names of groups that open. */
  .single-name {
    flex: none;
    padding: 6px 4px 6px 24px;
  }
  .head.single {
    justify-content: flex-start;
  }
  .head.single :global(.toggle) {
    margin-left: auto;
  }
  .dim {
    margin-left: auto;
    overflow: hidden;
    color: var(--dim);
    font-size: var(--text-hint);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .head.single .dim {
    margin-left: 0;
  }
  .kids {
    padding-left: 24px;
  }
  .kids :global(.row:last-child) {
    border-bottom: 0;
  }
</style>
