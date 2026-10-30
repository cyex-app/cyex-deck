<!-- V108-N18 "While you were away": what happened since you left, over the bottom of the chat.
     Jump goes to "New since you left"; each mention or moment goes to its line when that
     is still loaded. Esc or the x closes it; the list hides it too (MessageList.svelte). -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';

  /** A mention or moment: what to show, and the line it sits on (null: no longer loaded). */
  export interface Row {
    key: string;
    text: string;
    line: string | null;
  }

  interface Props {
    /** "7 min" */
    away: string;
    /** "2 mentions · 1 raid" */
    summary: string;
    rows: Row[];
    /** Whether the divider is in the list to jump to. */
    canJump: boolean;
    /** The budget cut lines from before the oldest one. */
    trimmed: boolean;
    onjump: () => void;
    onpick: (id: string) => void;
    onclose: () => void;
  }

  let { away, summary, rows, canJump, trimmed, onjump, onpick, onclose }: Props = $props();
</script>

<section class="card" aria-label="While you were away">
  <header>
    <span class="title">While you were away <span class="time">{away}</span></span>
    {#if canJump}<button class="jump" onclick={onjump}>Jump</button>{/if}
    <button class="x" aria-label="Close" onclick={onclose}><Icon name="x" size={13} /></button>
  </header>
  {#if summary}<p class="sum">{summary}</p>{/if}
  {#each rows as r (r.key)}
    {#if r.line}
      <button class="row" onclick={() => onpick(r.line!)}>{r.text}</button>
    {:else}
      <span class="row gone">{r.text}</span>
    {/if}
  {/each}
  {#if trimmed}<p class="note">Older lines were cut.</p>{/if}
</section>

<style>
  .card {
    position: absolute;
    left: 8px;
    right: 8px;
    bottom: var(--catch-bottom, 8px);
    max-width: 420px;
    margin-inline: auto;
    padding: 6px 8px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-pop);
    font-size: 0.8rem;
    line-height: 1.35;
  }
  header {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .title {
    flex: 1;
    min-width: 0;
    font-weight: 600;
  }
  .time {
    color: var(--dim);
    font-weight: 400;
    font-variant-numeric: tabular-nums;
  }
  .sum,
  .note {
    margin: 2px 0 0;
    color: var(--muted);
  }
  .note {
    color: var(--dim);
  }
  button {
    border-radius: var(--radius);
  }
  .jump {
    height: 22px;
    padding: 0 8px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    font-weight: 600;
  }
  .jump:hover,
  .x:hover,
  button.row:hover {
    background: var(--elevated-2);
  }
  .x {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    color: var(--muted);
  }
  .row {
    display: block;
    width: 100%;
    margin-top: 2px;
    padding: 2px 4px;
    overflow: hidden;
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
  }
  .row.gone {
    color: var(--dim);
  }
</style>
