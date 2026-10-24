<!-- Account manager (1.3, 1.23) and the two ways to add an account (V102-19): Sign in
     with Twitch (1.5), then optionally the GQL tools on the account card; or sign in with
     a code for the GQL tools alone. A flag on a greyed-out tool lands here, on the sign-in
     that unlocks it (V102-20). -->
<script lang="ts">
  import SectionHead from '../SectionHead.svelte';
  import { tick } from 'svelte';

  import Button from '../../../components/Button.svelte';
  import Icon from '../../../components/Icon.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import type { Place } from '../../../ipc/bindings/Place';
  import AccountCard from '../../accounts/AccountCard.svelte';
  import DeviceCode from '../../accounts/DeviceCode.svelte';
  import { signIn } from '../../accounts/signIn.svelte';
  import { accounts } from '../../../lib/accounts.svelte';
  import { signInTarget } from '../../../lib/gate';
  import { gate } from '../../../lib/gate.svelte';
  import { scrollBehavior } from '../../../lib/motion';
  import { scrollWithin } from '../../../lib/scrollWithin';
  import { settingsPlace } from '../place.svelte';

  // V107-Mo A removed account's open profile editor isn't remembered.
  $effect(() => settingsPlace.prune('accounts', 'profile:', accounts.list.map((a) => a.userId)));

  const PLACES: { value: Place; label: string }[] = [
    { value: 'browser', label: 'Browser' },
    { value: 'app', label: 'App window' },
  ];

  let twitchWay = $state<HTMLElement>();
  let codeWay = $state<HTMLElement>();
  let cards = $state<HTMLElement>();
  /** The way a flag pointed at, lit up for a moment. */
  let lit = $state<'twitch' | 'code' | null>(null);

  $effect(() => {
    if (!gate.asks || !gate.want) return;
    const target = signInTarget(gate.want, accounts.current);
    gate.want = null;
    void show(target);
  });

  async function show(target: 'account' | 'twitch' | 'code') {
    if (target === 'account' && accounts.current) signIn.offer = accounts.current.userId;
    await tick();
    const el = target === 'account' ? cards : target === 'twitch' ? twitchWay : codeWay;
    if (el) scrollWithin(el, 'center', scrollBehavior());
    if (target === 'account') return;
    lit = target;
    setTimeout(() => (lit = null), 1600);
  }
</script>

<SectionHead title="Accounts" level={2}>Who you chat and follow as. Each tab can use its own account.</SectionHead>

{#if signIn.kind}<DeviceCode />{/if}

<div bind:this={cards} data-setting="accounts.list accounts.defaultId accounts:profile">
  {#each accounts.list as account (account.userId)}
    <AccountCard {account} />
  {:else}
    <p class="note">No accounts yet. You can watch streams and read chat without one.</p>
  {/each}
</div>

<SectionHead title={accounts.list.length ? 'Add another account' : 'Add an account'} />
<div class="ways">
  <div class="way" class:lit={lit === 'twitch'} bind:this={twitchWay} data-setting="accounts:twitch">
    <Icon name="login" size={18} />
    <div class="text">
      <strong>Sign in with Twitch</strong>
      <span>Chat, mod tools, your follows, payment methods and subscriptions. The extra Twitch features can be added after.</span>
    </div>
    <Button variant="primary" small disabled={signIn.kind !== null} onclick={() => signIn.start('helix')}>
      Sign in
    </Button>
  </div>
  <div class="way" class:lit={lit === 'code'} bind:this={codeWay} data-setting="accounts:code">
    <Icon name="key" size={18} />
    <div class="text">
      <strong>Sign in with a code</strong>
      <span>Approve a short code on Twitch. Chat and the extra Twitch features: the miner, following, channel points, GIFs and power-ups. Payment methods and subscriptions need Sign in with Twitch too.</span>
    </div>
    <Button small disabled={signIn.kind !== null} onclick={() => signIn.start('gql')}>Get a code</Button>
  </div>
  <!-- V108-02 -->
  <div class="place">
    <span>Open in</span>
    <Segmented label="Where sign-ins open" options={PLACES} value={signIn.placeFor(null)} onchange={(p) => signIn.pick(p)} />
    <span class="hint">{signIn.placeFor(null) === 'app' ? 'Starts signed out, so you can pick any account.' : 'Uses the account your browser is signed in to.'}</span>
  </div>
</div>

<style>
  .note {
    margin: 4px 0 14px;
    color: var(--dim);
  }
  .ways {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 18px;
  }
  .way {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    transition: border-color var(--fast);
  }
  .place {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 10px;
    padding: 2px 2px 0;
    font-size: var(--text-sub);
  }
  .place .hint {
    color: var(--muted);
  }
  .way.lit {
    border-color: var(--accent-border);
  }
  .way > :global(svg) {
    flex: none;
    color: var(--dim);
  }
  .text {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .text span {
    color: var(--muted);
    font-size: var(--text-sub);
  }
</style>
