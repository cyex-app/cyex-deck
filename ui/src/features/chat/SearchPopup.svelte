<!-- F1 Search a chat with Chatterino's operators (`from:`, `has:link`, `badge:`,
     `regex:`, `is:` …), or every open chat at once (C29). Results come from Rust's
     buffers, newest at the bottom. -->
<script lang="ts">
  import { onMount } from 'svelte';

  import Checkbox from '../../components/Checkbox.svelte';
  import Icon from '../../components/Icon.svelte';
  import type { FeedLine } from '../../ipc/bindings/FeedLine';
  import { chatxIpc } from '../../ipc/chatx';
  import { lineClick } from './clicks';
  import Line from './Line.svelte';

  interface Props {
    view: number;
    channel: string;
    query?: string;
    all?: boolean;
    onclose: () => void;
  }

  let { view, channel, query = '', all = false, onclose }: Props = $props();

  // The popup starts from what it was opened with; typing takes over from there.
  // svelte-ignore state_referenced_locally
  let text = $state(query);
  // svelte-ignore state_referenced_locally
  let everywhere = $state(all);
  let results = $state.raw<FeedLine[]>([]);
  let input = $state<HTMLInputElement>();
  let list = $state<HTMLElement>();
  let timer: ReturnType<typeof setTimeout> | undefined;

  onMount(() => input?.focus());

  $effect(() => {
    const q = text;
    const scope = everywhere ? null : view;
    clearTimeout(timer);
    timer = setTimeout(() => {
      chatxIpc
        .search(scope, q)
        .then((r) => {
          results = r;
          requestAnimationFrame(() => list && (list.scrollTop = list.scrollHeight));
        })
        .catch(() => (results = []));
    }, 150);
    return () => clearTimeout(timer);
  });

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.stopPropagation();
      onclose();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="search" {onkeydown}>
  <div class="bar">
    <Icon name="search" size={14} />
    <input
      bind:this={input}
      bind:value={text}
      class="field"
      placeholder="Search · from:name has:link badge:mod regex:…"
      spellcheck="false"
    />
    <span class="all"><Checkbox label="All chats" checked={everywhere} onchange={(v) => (everywhere = v)} /></span>
    <button class="x" aria-label="Close" onclick={onclose}><Icon name="x" size={14} /></button>
  </div>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="results" bind:this={list} onclick={lineClick}>
    {#each results as r (`${r.channel}/${r.message.id}`)}
      <div data-channel={r.channel}>
        {#if everywhere}<div class="chan">#{r.channel}</div>{/if}
        <Line m={r.message} channel={r.channel} />
      </div>
    {:else}
      <p class="muted">{text.trim() ? 'Nothing found.' : `Type to search ${everywhere ? 'every open chat' : channel}.`}</p>
    {/each}
  </div>
  {#if results.length}<p class="count">{results.length} found</p>{/if}
</div>

<style>
  .search {
    position: absolute;
    inset: 0;
    z-index: 5;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
    color: var(--dim);
  }
  .bar .field {
    flex: 1;
    min-width: 0;
    height: 28px;
  }
  .all {
    color: var(--muted);
    font-size: 0.78rem;
    white-space: nowrap;
  }
  .x {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--dim);
  }
  .x:hover {
    color: var(--text);
  }
  .results {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }
  .chan {
    padding: 4px 10px 0;
    color: var(--dim);
    font-size: 0.72rem;
    font-weight: 700;
  }
  .muted {
    padding: 8px 12px;
    color: var(--dim);
    font-size: 0.85rem;
  }
  .count {
    margin: 0;
    padding: 4px 10px;
    border-top: 1px solid var(--border);
    color: var(--dim);
    font-size: 0.75rem;
  }
</style>
