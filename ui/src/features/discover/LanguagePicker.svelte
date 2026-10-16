<!-- H4 Which languages discover lists. Remembered in settings. -->
<script lang="ts">
  import Checkbox from '../../components/Checkbox.svelte';
  import Icon from '../../components/Icon.svelte';
  import Popover from '../../components/Popover.svelte';
  import { settings } from '../../lib/settings.svelte';
  import { LANGUAGES, languageLabel } from './languages';

  let open = $state(false);

  const picked = $derived(settings.v.browse.languages);

  function set(code: string, on: boolean) {
    const languages = on ? [...picked, code] : picked.filter((c) => c !== code);
    void settings.patch('browse', { languages });
  }
</script>

<div class="wrap">
  <button class="pick" class:on={picked.length > 0} onclick={() => (open = !open)} aria-haspopup="dialog" aria-expanded={open}>
    <Icon name="globe" size={14} />{languageLabel(picked)}<Icon name="chevronDown" size={13} />
  </button>
  <Popover bind:open align="end" label="Languages">
    <div class="list">
      <button class="all" disabled={!picked.length} onclick={() => void settings.patch('browse', { languages: [] })}>
        All languages
      </button>
      {#each LANGUAGES as [code, name] (code)}
        <div class="item"><Checkbox checked={picked.includes(code)} label={name} onchange={(on) => set(code, on)} /></div>
      {/each}
    </div>
  </Popover>
</div>

<style>
  .wrap {
    position: relative;
  }
  .pick {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 8px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.8rem;
    white-space: nowrap;
  }
  .pick:hover,
  .pick.on {
    color: var(--text);
  }
  .list {
    display: flex;
    flex-direction: column;
    max-height: min(420px, 60vh);
    overflow-y: auto;
    padding: 2px;
  }
  .item {
    padding: 4px 8px;
    border-radius: var(--radius);
    font-size: 0.85rem;
  }
  .item:hover {
    background: var(--surface-hover);
  }
  .all {
    margin-bottom: 4px;
    padding: 6px 8px;
    background: none;
    border: 0;
    border-bottom: 1px solid var(--border);
    color: var(--accent-text);
    text-align: left;
    font-size: 0.85rem;
  }
  .all:disabled {
    color: var(--dim);
  }
</style>
