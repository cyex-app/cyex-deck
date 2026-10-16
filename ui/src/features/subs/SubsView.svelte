<!-- 1.28 Your subscriptions without twitch.tv: active ones with renew dates, gifts and
     Prime, and the ones that ran out, to resub. Paying and payment methods open Twitch's
     own pages in the app's Twitch window (1.27, 3.8). -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';
  import ConfirmButton from '../../components/ConfirmButton.svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import GateFlag from '../../components/GateFlag.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import type { MySubs } from '../../ipc/bindings/MySubs';
  import type { SubChannel } from '../../ipc/bindings/SubChannel';
  import { subsIpc, type TwitchPage } from '../../ipc/subs';
  import { accounts } from '../../lib/accounts.svelte';
  import { formatDate } from '../../lib/format';
  import { latest } from '../../lib/latest';
  import { toast } from '../../lib/toast.svelte';
  import { canStop, months, standing, tierLabel } from './labels';

  interface Props {
    onchannel: (c: SubChannel) => void;
    onsignin: () => void;
  }

  let { onchannel, onsignin }: Props = $props();

  let data = $state.raw<MySubs | null>(null);
  let error = $state('');
  let loading = $state(false);

  const account = $derived(accounts.current?.userId ?? null);
  /** V102-20 The tab needs Sign in with Twitch; the list needs GQL on top. */
  const twitch = $derived(account !== null && accounts.can('wallet', account));
  const can = $derived(twitch && account !== null && accounts.can('subs', account));

  $effect(() => {
    if (can) void load();
  });

  const seq = latest();
  let dataFor: string | null = null;

  async function load() {
    if (!account) return;
    const live = seq.begin();
    const id = account;
    // Another account's list never shows under this one.
    if (dataFor !== id) data = null;
    loading = true;
    error = '';
    try {
      const d = await subsIpc.mine(id);
      if (live()) {
        data = d;
        dataFor = id;
      }
    } catch (e) {
      if (live()) error = String(e);
    } finally {
      if (live()) loading = false;
    }
  }

  function page(kind: TwitchPage, login: string | null = null) {
    if (account) subsIpc.page(account, kind, login).catch((e) => toast.error(String(e)));
  }

  async function stop(id: string, name: string) {
    if (!account) return;
    try {
      await subsIpc.stopRenewal(account, id);
      toast.info(`${name} won’t renew.`);
      await load();
    } catch (e) {
      toast.error(String(e));
    }
  }
</script>

<div class="subs">
  <header>
    <h2>Subscriptions</h2>
    {#if account && twitch}
      <Button small icon="card" onclick={() => page('wallet')}>Payment methods</Button>
      <Button small variant="ghost" onclick={() => page('subscriptions')}>Twitch’s page</Button>
      {#if can}<IconButton icon="refresh" label="Refresh" onclick={() => void load()} disabled={loading} />{/if}
    {/if}
  </header>
  {#if !account}
    <div class="none">
      <EmptyState icon="star" text="Sign in to see your subscriptions." />
      <Button variant="primary" icon="login" onclick={onsignin}>Sign in</Button>
    </div>
  {:else if !twitch}
    <div class="none">
      <EmptyState icon="star" text="Subscriptions and payment methods need Sign in with Twitch." />
      <GateFlag kind="helix" />
    </div>
  {:else if !can}
    <div class="none">
      <EmptyState icon="star" text="Your subs show here with a GQL sign-in. Twitch’s page works either way." />
      <GateFlag kind="gql" />
      <Button onclick={() => page('subscriptions')}>Open Twitch’s page</Button>
    </div>
  {:else if error}
    <p class="note err">{error}</p>
  {:else if !data}
    <p class="note">Loading…</p>
  {:else}
    <div class="body">
      <h3>Active <span class="n">{data.active.length}</span></h3>
      {#each data.active as s (s.id)}
        <div class="row">
          <button class="who" onclick={() => onchannel(s.channel)}>
            <Avatar src={s.channel.avatar} name={s.channel.name} size={32} />
            <span class="text">
              <span class="name">{s.channel.name}</span>
              <span class="meta">
                {tierLabel(s.tier)}{#if s.months} · {months(s.months)}{/if} · {standing(s)}
                {#if s.platform && s.platform !== 'WEB'} · bought on {s.platform.toLowerCase()}{/if}
              </span>
            </span>
          </button>
          <Button small onclick={() => page('subscribe', s.channel.login)}>Manage</Button>
          {#if canStop(s)}
            <ConfirmButton
              label="Stop renewal"
              confirm="Click again: it runs out, no refund"
              variant="ghost"
              onconfirm={() => void stop(s.id, s.channel.name)}
            />
          {/if}
        </div>
      {:else}
        <p class="note">No subs right now.</p>
      {/each}
      {#if data.expired.length}
        <h3>Ran out <span class="n">{data.expired.length}</span></h3>
        {#each data.expired as s (s.channel.id)}
          <div class="row">
            <button class="who" onclick={() => onchannel(s.channel)}>
              <Avatar src={s.channel.avatar} name={s.channel.name} size={32} />
              <span class="text">
                <span class="name">{s.channel.name}</span>
                <span class="meta">
                  {tierLabel(s.tier)}{#if s.months} · {months(s.months)}{/if}{#if s.endedAt} · ended {formatDate(new Date(s.endedAt))}{/if}
                </span>
              </span>
            </button>
            <Button small variant="primary" onclick={() => page('subscribe', s.channel.login)}>Resubscribe</Button>
          </div>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .subs {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }
  h2 {
    flex: 1;
    margin: 0;
    font-size: 16px;
  }
  .none {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
  }
  .body {
    flex: 1;
    overflow-y: auto;
    padding: 8px 16px 16px;
  }
  h3 {
    margin: 14px 0 6px;
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .n {
    color: var(--dim);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 4px;
    border-radius: var(--radius);
  }
  .row:hover {
    background: var(--row-hover);
  }
  .who {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    background: none;
    border: 0;
    text-align: left;
  }
  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .name {
    font-weight: 600;
  }
  .meta {
    color: var(--muted);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .note {
    margin: 16px;
    color: var(--dim);
  }
  .err {
    color: var(--red);
  }
</style>
