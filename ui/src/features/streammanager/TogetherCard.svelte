<!-- 1.35 Stream Together (Twitch's Guest Star): start a session, invite channels, put guests
     who are ready on stream or take them off, end it. Guests join from their own Twitch;
     joining someone else's session happens on Twitch too. D6 Creators asking to join get an
     answer here, and guests can show on stream as soon as they're set up. -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import type { SmTogether } from '../../ipc/bindings/SmTogether';
  import { smIpc } from '../../ipc/streammanager';
  import { toast } from '../../lib/toast.svelte';
  import Card from '../../components/PanelCard.svelte';
  import ConfirmButton from '../../components/ConfirmButton.svelte';

  /** Twitch allows the host plus five guests. */
  const SLOTS = 5;

  let together = $state<SmTogether | null>(null);
  let invite = $state('');
  let busy = $state(false);

  const session = $derived(together?.session ?? null);
  const guests = $derived(session?.guests ?? []);
  /** The lowest guest slot nobody holds, or null when full. */
  const freeSlot = $derived.by(() => {
    const taken = new Set(guests.map((g) => g.slotId));
    for (let i = 1; i <= SLOTS; i++) if (!taken.has(String(i))) return String(i);
    return null;
  });

  function load() {
    smIpc
      .together()
      .then((s) => (together = s))
      .catch((e) => toast.error(String(e)));
  }
  load();

  // D6 Join requests come and go on Twitch's side.
  $effect(() => {
    const off = smIpc.onTogether(load);
    return () => void off.then((f) => f());
  });

  function autoAdd(on: boolean) {
    if (together) together = { ...together, autoAdd: on };
    smIpc.autoAdd(session?.id ?? null, on).catch((e) => {
      toast.error(String(e));
      load();
    });
  }

  /** Runs an action, then reads the session again. */
  function act(p: Promise<void>, done?: string) {
    busy = true;
    p.then(() => {
      if (done) toast.info(done);
      load();
    })
      .catch((e) => toast.error(String(e)))
      .finally(() => (busy = false));
  }

  function send() {
    const login = invite.trim().replace(/^@/, '');
    if (!session || !login) return;
    act(smIpc.togetherInvite(session.id, login), `Invited ${login}.`);
    invite = '';
  }
</script>

<Card title="Stream Together">
  {#snippet tools()}
    <IconButton icon="refresh" label="Refresh" size={14} onclick={load} />
  {/snippet}
  {#if together?.requests.length}
    <span class="label">Asking to join</span>
    <ul>
      {#each together.requests as r (r.userId)}
        <li>
          <Avatar src={r.avatar} name={r.name} size={20} />
          <span class="name" title={r.message || undefined}>{r.name}</span>
          <button class="link" disabled={busy} onclick={() => act(smIpc.joinAnswer(r.userId, true), `${r.name} can join.`)}>Let in</button>
          <button class="link" disabled={busy} onclick={() => act(smIpc.joinAnswer(r.userId, false))}>Decline</button>
        </li>
      {/each}
    </ul>
  {/if}
  {#if together && together.autoAdd !== null}
    <div class="line">
      <Toggle label="Show guests right away" checked={together.autoAdd} onchange={autoAdd} />
      <span>Show guests right away</span>
    </div>
  {/if}
  {#if !together}
    <p class="note">Loading…</p>
  {:else if !session}
    <p class="note">Stream with up to five others in one layout. Guests join from their own Twitch.</p>
    <div class="line"><Button small disabled={busy} onclick={() => act(smIpc.togetherStart(), 'Session started.')}>Start a session</Button></div>
  {:else}
    <ul>
      {#each guests as g (g.userId)}
        <li>
          <span class="slot">{g.slotId === '0' ? 'Host' : `Slot ${g.slotId}`}</span>
          <span class="dot" class:live={g.isLive} title={g.isLive ? 'On stream' : 'Not on stream'}></span>
          <span class="name">{g.userName || g.userLogin}</span>
          {#if g.slotId !== '0'}
            <button class="link" disabled={busy} onclick={() => act(smIpc.togetherSlot(session.id, g.userId, g.slotId, true))}>Take off</button>
          {/if}
        </li>
      {/each}
    </ul>

    {#if together.invites.length}
      <span class="label">Invited</span>
      <ul>
        {#each together.invites as i (i.userId)}
          <li>
            <span class="name">{i.name}</span>
            <span class="status">{i.status === 'READY' ? 'Ready' : i.status === 'ACCEPTED' ? 'Joining' : 'Invited'}</span>
            {#if i.status === 'READY' && freeSlot}
              <button class="link" disabled={busy} onclick={() => act(smIpc.togetherSlot(session.id, i.userId, freeSlot!, false))}>Put on stream</button>
            {/if}
            <button class="link" disabled={busy} onclick={() => act(smIpc.togetherUninvite(session.id, i.userId))}>Cancel</button>
          </li>
        {/each}
      </ul>
    {/if}

    <form
      class="line"
      onsubmit={(e) => {
        e.preventDefault();
        send();
      }}
    >
      <input class="field grow" placeholder="Invite a channel" bind:value={invite} />
      <Button small type="submit" disabled={busy || !invite.trim()}>Invite</Button>
    </form>
    <div class="line">
      <ConfirmButton
        label="End session"
        confirm="Click again to end it"
        variant="ghost"
        disabled={busy}
        onconfirm={() => act(smIpc.togetherEnd(session.id), 'Session ended.')}
      />
    </div>
  {/if}
</Card>

<style>
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  li {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.82rem;
    min-height: 24px;
  }
  .slot {
    width: 44px;
    color: var(--dim);
    font-size: 0.75rem;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--border-strong);
  }
  .dot.live {
    background: var(--red);
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .status {
    color: var(--muted);
    font-size: 0.76rem;
  }
  .grow {
    flex: 1;
  }
  .link {
    background: none;
    border: 0;
    padding: 0;
    color: var(--muted);
    font-size: 0.78rem;
    cursor: pointer;
  }
  .link:hover:not(:disabled) {
    color: var(--text);
  }
</style>
