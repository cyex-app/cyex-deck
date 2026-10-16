<!-- 1.26 Twitch's bell in the top bar: the unseen count, then the list (yours, and the
     creator one for streamers), read, dismiss, mark all read. A click opens a channel as a
     tab and any other Twitch page in the app's Twitch window. -->
<script lang="ts">
  import { untrack } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Popover from '../../components/Popover.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import type { BellItem } from '../../ipc/bindings/BellItem';
  import type { BellList } from '../../ipc/bindings/BellList';
  import { ago } from '../../lib/format';
  import { bell } from './bell.svelte';
  import { follow } from './follow';
  import Loading from '../../components/Loading.svelte';

  let { onchannel }: { onchannel: (login: string) => void } = $props();

  let open = $state(false);
  const LISTS: { value: BellList; label: string }[] = [
    { value: 'viewer', label: 'For you' },
    { value: 'creator', label: 'Your channel' },
  ];

  $effect(() => {
    void bell.start();
  });

  $effect(() => {
    if (open) untrack(() => void bell.opened());
  });

  function click(item: BellItem) {
    void bell.read(item);
    if (!item.url) return;
    open = false;
    follow(item.url, onchannel);
  }

  /** Twitch's markdown is just `**bold**`. */
  const parts = (md: string) => md.split('**').map((text, i) => ({ text, bold: i % 2 === 1 }));
</script>

{#if bell.available}
  <div class="wrap">
    <IconButton
      icon="bell"
      label="Notifications"
      hint={bell.unseen ? `${bell.unseen} new on Twitch` : 'Your Twitch notifications.'}
      active={open}
      onclick={() => (open = !open)}
    />
    {#if bell.unseen}<span class="count">{bell.unseen > 99 ? '99+' : bell.unseen}</span>{/if}
    <Popover bind:open>
      <div class="panel">
        <header>
          <Segmented options={LISTS} value={bell.list} label="Which notifications" onchange={(l) => void bell.load(l)} />
          <button class="all" onclick={() => void bell.readAll()} disabled={!bell.items.some((i) => !i.read)}>
            <Icon name="checkCheck" size={14} />Mark all read
          </button>
        </header>
        <div class="list">
          {#each bell.items as item (item.id)}
            <div class="item" class:unread={!item.read}>
              <button class="main" onclick={() => click(item)}>
                {#if item.thumbnail}<img src={item.thumbnail} alt="" loading="lazy" />{:else}<span class="ph"
                    ><Icon name="bell" size={16} /></span
                  >{/if}
                <span class="text">
                  <span class="body"
                    >{#each parts(item.body) as p, i (i)}{#if p.bold}<b>{p.text}</b>{:else}{p.text}{/if}{/each}</span
                  >
                  <span class="when">{ago(Date.parse(item.createdAt))}</span>
                </span>
              </button>
              <button class="x" aria-label="Remove" onclick={() => void bell.remove(item.id)}><Icon name="x" size={13} /></button>
            </div>
          {:else}
            {#if bell.loading}<Loading />{:else}<p class="empty">Nothing here.</p>{/if}
          {/each}
          {#if bell.cursor}
            <button class="more" disabled={bell.loading} onclick={() => void bell.load(bell.list, true)}>Show more</button>
          {/if}
        </div>
      </div>
    </Popover>
  </div>
{:else}
  <!-- V102-20 Greyed out until the default account has a GQL sign-in. -->
  <IconButton icon="bell" label="Notifications" lock="gql" />
{/if}

<style>
  .wrap {
    position: relative;
  }
  .count {
    position: absolute;
    top: 1px;
    right: 0;
    min-width: 15px;
    height: 15px;
    padding: 0 4px;
    border-radius: 8px;
    background: var(--red);
    color: #fff;
    font-size: 10px;
    font-weight: 700;
    line-height: 15px;
    text-align: center;
    pointer-events: none;
    box-shadow: 0 0 0 2px var(--bg);
  }
  .panel {
    width: 360px;
    max-height: min(560px, 70vh);
    display: flex;
    flex-direction: column;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 6px 8px;
    border-bottom: 1px solid var(--border);
  }
  .all {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 4px 6px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 12px;
  }
  .all:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--text);
  }
  .all:disabled {
    opacity: 0.5;
  }
  .list {
    overflow-y: auto;
    padding: 4px 0;
  }
  .item {
    position: relative;
    display: flex;
    border-radius: var(--radius);
  }
  .item:hover {
    background: var(--surface-hover);
  }
  .main {
    flex: 1;
    display: flex;
    gap: 10px;
    padding: 8px 28px 8px 8px;
    background: none;
    border: 0;
    text-align: left;
    min-width: 0;
  }
  img,
  .ph {
    width: 36px;
    height: 36px;
    flex: none;
    border-radius: 50%;
    object-fit: cover;
    display: grid;
    place-items: center;
    background: var(--surface);
    color: var(--dim);
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .body {
    color: var(--muted);
    font-size: 13px;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }
  .unread .body {
    color: var(--text);
  }
  .unread::before {
    content: '';
    position: absolute;
    left: 2px;
    top: 50%;
    width: 4px;
    height: 4px;
    margin-top: -2px;
    border-radius: 50%;
    background: var(--accent-text);
  }
  .when {
    color: var(--dim);
    font-size: 11px;
  }
  .x {
    position: absolute;
    top: 6px;
    right: 4px;
    display: none;
    padding: 3px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--dim);
  }
  .item:hover .x {
    display: flex;
  }
  .x:hover {
    color: var(--text);
    background: var(--elevated-2);
  }
  .empty {
    margin: 18px 0;
    color: var(--dim);
    text-align: center;
  }
  .more {
    width: 100%;
    padding: 8px;
    background: none;
    border: 0;
    color: var(--accent-text);
  }
</style>
