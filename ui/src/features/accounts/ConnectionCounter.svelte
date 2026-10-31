<!-- Joined-chat counter in the top bar (1.13, D7): the chats the account picked here has open.
     Turns yellow and names them when Twitch refused some (V105-05). A click lists them per
     account, chat only or with a stream. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import Popover from '../../components/Popover.svelte';
  import { accounts } from '../../lib/accounts.svelte';
  import { tabs, tiles } from '../tabs/tabs.svelte';
  import { counterView } from './counter';
  import { joinedGroups } from './joined';

  let open = $state(false);
  const name = (id: string) => accounts.list.find((a) => a.userId === id)?.displayName ?? id;

  const v = $derived(counterView(accounts.connections, accounts.current?.userId ?? '', name));
  const groups = $derived(
    open
      ? joinedGroups(
          accounts.connections,
          accounts.current?.userId ?? '',
          name,
          tabs.list.flatMap((t) => tiles(t).filter((x) => !x.feed && x.show.player).map((x) => x.channel.login)),
        )
      : [],
  );
</script>

{#if accounts.current && !v.hidden}
  <!-- V1051-05 Its own icon (the bubble read as Whispers) and a spoken name. -->
  <div class="wrap">
    <button
      class="counter"
      class:warn={v.warn}
      class:active={open}
      aria-label={v.aria}
      aria-haspopup="dialog"
      aria-expanded={open}
      onclick={() => (open = !open)}
      use:tooltip={{ label: v.label, hint: v.hint }}
    >
      <Icon name={v.warn ? 'alert' : 'radio'} size={16} />
      {v.text}
    </button>
    <Popover bind:open label="Joined chats">
      <div class="panel">
        {#each groups as g (g.userId)}
          <section>
            <h3>{g.name}</h3>
            <ul>
              {#each g.channels as c (c.login)}
                <li><span class="login">{c.login}</span><span class="kind" class:stream={c.stream}>{c.stream ? 'Chat and stream' : 'Chat only'}</span></li>
              {/each}
            </ul>
          </section>
        {:else}
          <p class="empty">No chats joined.</p>
        {/each}
      </div>
    </Popover>
  </div>
{/if}

<style>
  .wrap {
    position: relative;
    margin-right: 4px;
  }
  .counter {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    height: 30px;
    min-width: 30px;
    padding: 0 8px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.78rem;
    font-variant-numeric: tabular-nums;
    transition:
      background var(--fast),
      color var(--fast);
  }
  .counter:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .counter:active {
    opacity: 0.6;
    transition: none;
  }
  .counter.active {
    color: var(--accent-text);
    background: var(--accent-dim);
  }
  .warn,
  .warn:hover {
    color: var(--yellow);
  }
  .panel {
    width: 260px;
    max-height: min(420px, 70vh);
    overflow-y: auto;
    padding: 4px 0;
  }
  section + section {
    margin-top: 6px;
  }
  h3 {
    margin: 0;
    padding: 4px 10px;
    color: var(--dim);
    font-size: 0.72rem;
    font-weight: 600;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    padding: 3px 10px;
    font-size: 0.82rem;
  }
  .login {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .kind {
    flex: none;
    color: var(--dim);
    font-size: 0.72rem;
  }
  .kind.stream {
    color: var(--muted);
  }
  .empty {
    margin: 0;
    padding: 10px;
    color: var(--dim);
    font-size: 0.82rem;
  }
</style>
