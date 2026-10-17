<!-- V106-17 Streamer mode bar: the main window, under the title bar, after a streaming app
     started (or was running at launch). Settings, Turn off, Close. No auto-hide. -->
<script lang="ts">
  import { fade, slide } from 'svelte/transition';

  import Button from '../components/Button.svelte';
  import Icon from '../components/Icon.svelte';
  import IconButton from '../components/IconButton.svelte';
  import { reduced } from '../lib/motion';
  import { streamer } from '../lib/streamer.svelte';

  let { onsettings, narrow = false }: { onsettings: () => void; narrow?: boolean } = $props();

  const text = $derived(
    streamer.bar === 'started'
      ? 'Streamer mode is on. Your streaming app just started.'
      : 'Streamer mode is on. Your streaming app is running.',
  );

  function key(e: KeyboardEvent) {
    if (e.key === 'Escape') streamer.closeBar();
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="bar"
  role="status"
  aria-live="polite"
  onkeydown={key}
  transition:slide={{ duration: reduced() ? 0 : 180 }}
>
  <div class="inner" transition:fade={{ duration: reduced() ? 0 : 120 }}>
    <Icon name="eyeOff" size={14} />
    <span class="text">{text}</span>
    {#if narrow}
      <IconButton icon="settings" label="Settings" onclick={onsettings} />
    {:else}
      <Button small variant="ghost" onclick={onsettings}>Settings</Button>
    {/if}
    <Button small variant="ghost" onclick={() => streamer.turnOff()}>Turn off</Button>
    <IconButton icon="x" label="Close" hint="Streamer mode stays on." onclick={() => streamer.closeBar()} />
  </div>
</div>

<style>
  .bar {
    flex: none;
    overflow: hidden;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    font-size: 0.85rem;
  }
  .inner {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 8px 0 12px;
    color: var(--muted);
  }
  .text {
    flex: 1;
    min-width: 120px;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--text);
  }
</style>
