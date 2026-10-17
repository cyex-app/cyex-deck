<!-- V1061-03 Q23 While the stream plays behind live, the chat says it's live. -->
<script lang="ts">
  import { chatBarText } from '../player/liveEdge';
  import { rewound } from '../player/rewound.svelte';

  let { channel }: { channel: string } = $props();
  const r = $derived(rewound.get(channel));
</script>

{#if r && !r.ended}<div class="live-chat-bar">{chatBarText(r.behind)}</div>{/if}

<style>
  .live-chat-bar {
    margin: 6px 8px 2px;
    padding: 4px 10px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
