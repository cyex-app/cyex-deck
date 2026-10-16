<!-- 1.28 Subscribing to one channel without twitch.tv: where you stand, the free Prime sub,
     sharing a resub. Paying (subscribe, resub, gift, change tier) opens Twitch's own page in
     the app's Twitch window. -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';
  import ConfirmButton from '../../components/ConfirmButton.svelte';
  import Dialog from '../../components/Dialog.svelte';
  import type { ChannelSub } from '../../ipc/bindings/ChannelSub';
  import { subsIpc, type TwitchPage } from '../../ipc/subs';
  import { toast } from '../../lib/toast.svelte';
  import { months, standing, tierLabel } from './labels';
  import ResubShare from './ResubShare.svelte';
  import { errorText } from '../../lib/errors';
  import Loading from '../../components/Loading.svelte';

  let { account, login, onclose }: { account: string; login: string; onclose: () => void } = $props();

  let info = $state<ChannelSub | null>(null);
  let error = $state('');

  $effect(() => {
    void load();
  });

  async function load() {
    try {
      info = await subsIpc.channel(account, login);
      if (!info) error = 'That channel is gone.';
    } catch (e) {
      error = errorText(e);
    }
  }

  function page(kind: TwitchPage) {
    subsIpc.page(account, kind, login).catch((e) => toast.error(errorText(e)));
  }

  async function prime() {
    if (!info) return;
    try {
      await subsIpc.prime(account, info.channel.id);
      toast.info(`Subscribed to ${info.channel.name} with Prime.`);
      await load();
    } catch (e) {
      toast.error(errorText(e));
    }
  }
</script>

<Dialog title="Subscribe" width={400} {onclose}>
  {#if error}
    <p class="note err">{error}</p>
  {:else if !info}
    <Loading />
  {:else}
    <div class="who">
      <Avatar src={info.channel.avatar} name={info.channel.name} size={36} />
      <div>
        <div class="name">{info.channel.name}</div>
        <div class="state">
          {#if info.sub}
            {tierLabel(info.sub.tier)}{#if info.sub.months} · {months(info.sub.months)}{/if} · {standing(info.sub)}
          {:else if !info.sells}
            This channel doesn’t have subs.
          {:else}
            Not subscribed
          {/if}
        </div>
      </div>
    </div>
    {#if info.sells}
      <div class="actions">
        <Button variant="primary" icon="star" onclick={() => page('subscribe')}>
          {info.sub ? 'Renew or change tier' : 'Subscribe'}
        </Button>
        {#if info.canPrime && !info.sub}
          <ConfirmButton label="Use my Prime sub" confirm="Click again to use it here" icon="gem" onconfirm={prime} />
        {/if}
        <Button icon="gift" onclick={() => page('gift')}>Gift a sub</Button>
      </div>
      <p class="hint">Paying happens on Twitch’s own page, here in the app.</p>
    {/if}
    {#if info.resub}
      <div class="resub">
        <ResubShare {account} {login} resub={info.resub} onshared={onclose} />
      </div>
    {/if}
  {/if}
</Dialog>

<style>
  .note {
    margin: 12px 0;
    color: var(--dim);
  }
  .err {
    color: var(--red);
  }
  .who {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 14px;
  }
  .name {
    font-weight: 700;
  }
  .state {
    color: var(--muted);
    font-size: 13px;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .hint {
    margin: 8px 0 0;
    color: var(--dim);
    font-size: 12px;
  }
  .resub {
    margin-top: 14px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }
</style>
