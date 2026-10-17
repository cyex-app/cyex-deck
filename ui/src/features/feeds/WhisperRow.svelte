<!-- V106-07 One whisper received this session. In streamer mode the text hides until its eye
     is pressed. -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import type { WhisperEntry } from '../../ipc/bindings/WhisperEntry';
  import { streamer } from '../../lib/streamer.svelte';
  import { whispers } from '../whispers/whispers.svelte';

  interface Props {
    entry: WhisperEntry;
    revealed: boolean;
    onreveal: (on: boolean) => void;
  }

  let { entry, revealed, onreveal }: Props = $props();

  const hidden = $derived(streamer.hides('hideWhispers'));
</script>

<div class="whisper">
  <Avatar src={entry.avatar} name={entry.name} size={20} />
  <div class="body">
    <b>{entry.name}</b>
    {#if hidden && !revealed}
      <span class="text hid">Hidden in streamer mode</span>
    {:else}
      <span class="text">{entry.text}</span>
    {/if}
  </div>
  {#if hidden && !revealed}
    <IconButton icon="eye" label="Show this message" onclick={() => onreveal(true)} />
  {:else if hidden}
    <IconButton icon="eyeOff" label="Hide it again" onclick={() => onreveal(false)} />
  {/if}
  <IconButton icon="whisper" label="Open this conversation" onclick={() => whispers.show(entry.login)} />
</div>

<style>
  .whisper {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .text {
    color: var(--muted);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .hid {
    font-style: italic;
  }
</style>
