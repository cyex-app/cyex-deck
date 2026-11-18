<!-- V111-60 Shown once, quietly, in Settings when Chatterino is on this PC and nothing came from
     it yet: a link to Settings › Import. Closing it, opening Import or importing hides it for
     good. Nothing is imported from here. -->
<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '../../../components/Button.svelte';
  import Icon from '../../../components/Icon.svelte';
  import { tooltip } from '../../../components/tooltip.svelte';
  import { settingsIpc } from '../../../ipc/settings';
  import { settings } from '../../../lib/settings.svelte';
  import { settingsJump } from '../menus/place.svelte';

  let found = $state(false);
  const shown = $derived(found && !settings.v.general.chatterinoOffered);

  onMount(() => {
    if (settings.v.general.chatterinoOffered) return;
    settingsIpc
      .chatterinoFolder()
      .then((f) => (found = !!f && (f.layout || f.settings || f.commands)))
      .catch(() => {});
  });

  function close() {
    void settings.patch('general', { chatterinoOffered: true });
  }
</script>

{#if shown}
  <div class="hint" role="note">
    <span>Coming from Chatterino? Bring your tabs, highlights and commands over in</span>
    <Button variant="link" hit onclick={() => (settingsJump.to = 'import')}>Import</Button>
    <button class="x" aria-label="Don’t show again" use:tooltip={{ label: 'Don’t show again' }} onclick={close}><Icon name="x" size={13} /></button>
  </div>
{/if}

<style>
  .hint {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px 6px;
    margin: 0 0 16px;
    padding: 8px 6px 8px 12px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--surface);
    font-size: var(--text-sub);
    color: var(--muted);
  }
  .x {
    margin-left: auto;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: 0;
    border-radius: 3px;
    background: none;
    color: var(--dim);
    cursor: pointer;
  }
  .x:hover {
    color: var(--text);
    background: var(--row-hover);
  }
  .x:active {
    opacity: 0.6;
    transition: none;
  }
  .x:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
</style>
