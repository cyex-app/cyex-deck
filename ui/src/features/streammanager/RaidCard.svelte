<!-- 1.35 Start a raid to another channel. Twitch counts down 90 s before it goes, and it
     can be canceled until then. D2 The countdown comes live from Twitch, "Raid now" works
     after 10 s, and Twitch's suggestions fill in the name (sign-in with a code). -->
<script lang="ts">
  import { everyShown } from '../../lib/awake';
  import { uniqueKeys } from '../../lib/keys';
  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';
  import ConfirmButton from '../../components/ConfirmButton.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import type { SmRaidPick } from '../../ipc/bindings/SmRaidPick';
  import { smIpc } from '../../ipc/streammanager';
  import { formatNumber } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import Card from '../../components/PanelCard.svelte';
  import { raidArmedDue, raidCountdown, raidMarkLabel } from './logic';
  import { sm } from './streammanager.svelte';
  import { errorText } from '../../lib/errors';

  const PICKS = 6;

  let target = $state('');
  /** Started here, until Twitch's first update comes. */
  let starting = $state<string | null>(null);
  let picks = $state<SmRaidPick[]>([]);
  const pickKeys = $derived(uniqueKeys(picks, (p) => p.login));
  let busy = $state(false);
  let now = $state(Date.now() / 1000);

  const login = $derived(target.trim().replace(/^@/, '').toLowerCase());
  const raid = $derived(sm.raid);
  const clock = $derived(raid ? raidCountdown(raid.createdAt, raid.goesAt, now) : null);

  $effect(() => {
    if (!raid) return;
    starting = null;
    // V110-F84 Not while the window is hidden.
    return everyShown(() => (now = Date.now() / 1000), 500);
  });

  // V110-N11 Raid now before the 10 s mark arms and fires at the mark; pressing the label undoes it. A raid
  // that ends or is replaced disarms.
  let armedFor = $state<string | null>(null);
  $effect(() => {
    if (armedFor && raid?.id !== armedFor) armedFor = null;
  });
  $effect(() => {
    if (!raidArmedDue(armedFor, raid, now) || busy) return;
    armedFor = null;
    act(smIpc.raidNow(), () => {});
  });
  function raidNow() {
    if (!raid) return;
    if (clock?.ready) act(smIpc.raidNow(), () => {});
    else armedFor = raid.id;
  }

  function loadPicks() {
    smIpc
      .raidPicks()
      .then((p) => (picks = p.slice(0, PICKS)))
      .catch(() => (picks = []));
  }
  loadPicks();

  function act(p: Promise<void>, after: () => void) {
    busy = true;
    p.then(after)
      .catch((e) => toast.error(errorText(e)))
      .finally(() => (busy = false));
  }

  function start() {
    // V110-F84 Enter twice is one raid request.
    if (!login || busy) return;
    const to = login;
    act(smIpc.raid(to), () => {
      starting = to;
      target = '';
    });
  }

  function cancel() {
    act(smIpc.unraid(), () => {
      starting = null;
      toast.success('Raid canceled.');
    });
  }
</script>

<Card title="Raid">
  {#snippet tools()}
    {#if !raid && !starting && picks.length}
      <IconButton icon="refresh" label="Other suggestions" size={14} onclick={loadPicks} />
    {/if}
  {/snippet}
  {#if raid && clock}
    <div class="target">
      <Avatar src={raid.targetAvatar} name={raid.targetName} size={32} />
      <div class="who">
        <strong>{raid.targetName}</strong>
        <span class="note">{formatNumber(raid.viewers)} going · {clock.left > 0 ? `in ${clock.left}s` : 'now'}</span>
      </div>
    </div>
    <div class="line">
      <!-- V110-A4 Both ask first: a raid sends everyone over. -->
      {#if armedFor}
        <Button onclick={() => (armedFor = null)}>{raidMarkLabel()}</Button>
      {:else}
        <ConfirmButton label="Raid now" confirm="Raid now?" disabled={busy} onconfirm={raidNow} />
      {/if}
      <ConfirmButton label="Cancel raid" confirm="Cancel raid?" variant="danger" disabled={busy} onconfirm={cancel} />
    </div>
  {:else if starting}
    <p class="note">Raiding {starting}. Twitch sends everyone over in about 90 seconds.</p>
    <div class="line"><ConfirmButton label="Cancel raid" confirm="Cancel raid?" variant="danger" disabled={busy} onconfirm={cancel} /></div>
  {:else}
    <form
      class="line"
      onsubmit={(e) => {
        e.preventDefault();
        start();
      }}
    >
      <input class="field grow" placeholder="Channel name" bind:value={target} />
      <Button small type="submit" disabled={!login || busy}>Raid</Button>
    </form>
    {#if picks.length}
      <span class="label">Suggested</span>
      <ul>
        {#each picks as p, i (pickKeys[i])}
          <li>
            <button class="pick" onclick={() => (target = p.login)}>
              <Avatar src={p.avatar} name={p.name} size={20} />
              <span class="name">{p.name}</span>
              <span class="meta">{formatNumber(p.viewers)}{p.category ? ` · ${p.category}` : ''}</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</Card>

<style>
  .grow {
    flex: 1;
  }
  .target {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .who {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .pick {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 3px 4px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--text);
    text-align: left;
  }
  .pick:hover {
    background: var(--row-hover);
  }
  .name {
    min-width: 0;
    overflow: hidden;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    min-width: 0;
    overflow: hidden;
    color: var(--dim);
    font-size: 0.78rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
