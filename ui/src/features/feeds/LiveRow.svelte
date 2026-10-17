<!-- V106-07 One live or offline notice. Click opens the channel; a channel without a tab
     gets an Add tab button (Stream and/or Chat). -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';
  import Popover from '../../components/Popover.svelte';
  import type { LiveEntry } from '../../ipc/bindings/LiveEntry';
  import type { TileShow } from '../../ipc/bindings/TileShow';
  import { notifyEvents } from '../../ipc/notify';
  import { accounts } from '../../lib/accounts.svelte';
  import { errorText } from '../../lib/errors';
  import { toast } from '../../lib/toast.svelte';
  import ShowPicker from '../tabs/ShowPicker.svelte';
  import { tabs, tiles } from '../tabs/tabs.svelte';

  let { entry }: { entry: LiveEntry } = $props();

  let open = $state(false);
  let show = $state<TileShow>({ ...tabs.lastShow });

  const hasTab = $derived(tabs.list.some((t) => tiles(t).some((x) => x.channel.id === entry.channelId)));
  const second = $derived(entry.live ? [entry.title, entry.game].filter(Boolean).join(' · ') : '');

  async function add() {
    try {
      await tabs.apply({
        op: 'add',
        channel: { id: entry.channelId, login: entry.login, name: entry.name },
        show,
        account: accounts.current?.userId ?? null,
        alias: null,
        into: null,
      });
      open = false;
    } catch (e) {
      toast.error(errorText(e));
    }
  }
</script>

<div class="live">
  <button class="row" onclick={() => void notifyEvents.open({ kind: 'channel', login: entry.login })}>
    <Avatar src={entry.avatar} name={entry.name} size={20} />
    <span class="text">
      <span class="line"><b class:off={!entry.live}>{entry.name}</b> {entry.live ? 'went live' : 'went offline'}</span>
      {#if second}<span class="line sub">{second}</span>{/if}
    </span>
  </button>
  {#if !hasTab}
    <div class="wrap">
      <Button small variant="ghost" icon="plus" onclick={() => (open = !open)}>Add tab</Button>
      <Popover bind:open align="end" role="dialog" label="Add a tab">
        <div class="panel">
          <ShowPicker bind:show />
          <Button small variant="primary" onclick={add}>Add tab</Button>
        </div>
      </Popover>
    </div>
  {/if}
</div>

<style>
  .live {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .row {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 4px;
    border: 0;
    border-radius: var(--radius);
    background: none;
    color: inherit;
    font: inherit;
    text-align: start;
    cursor: pointer;
  }
  .row:hover,
  .row:active {
    background: var(--surface-hover);
  }
  .row:focus-visible {
    outline: 2px solid var(--text);
    outline-offset: -2px;
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .line {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .sub {
    color: var(--muted);
    font-size: 0.76rem;
  }
  .off {
    color: var(--muted);
  }
  .wrap {
    position: relative;
    flex-shrink: 0;
  }
  .panel {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 10px;
  }
</style>
