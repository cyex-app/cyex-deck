<!-- Channel points (1.9, 1.40): balance next to the message box and the rewards menu,
     with power-ups paid in Bits (1.42) on a second page. Redeeming needs the account's
     GQL sign-in; without one the menu only lists rewards. The balance follows Hermes
     (points earned, chests, 08). -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import GateFlag from '../../components/GateFlag.svelte';
  import Icon from '../../components/Icon.svelte';
  import Popover from '../../components/Popover.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import { chatIpc } from '../../ipc/chat';
  import { minerIpc } from '../../ipc/miner';
  import type { Points } from '../../ipc/bindings/Points';
  import { accounts } from '../../lib/accounts.svelte';
  import { formatNumber, formatShort } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import EmotePick from './EmotePick.svelte';
  import PowerUps from './PowerUps.svelte';
  import type { ChatRoom } from './room.svelte';

  let { room }: { room: ChatRoom } = $props();

  interface Choice {
    id: string;
    title: string;
    cost: number;
    color: string;
    image: string | null;
    prompt: string | null;
    needsText: boolean;
    /** The reward picks an emote. */
    emote?: 'unlock' | 'modify';
    available: boolean;
  }

  /** Built-in rewards this app can redeem. */
  const AUTOMATIC: Record<string, { title: string; needsText: boolean; emote?: 'unlock' | 'modify' }> = {
    SEND_HIGHLIGHTED_MESSAGE: { title: 'Highlight my message', needsText: true },
    SINGLE_MESSAGE_BYPASS_SUB_MODE: { title: 'Send a message in sub-only mode', needsText: true },
    RANDOM_SUB_EMOTE_UNLOCK: { title: 'Unlock a random sub emote', needsText: false },
    CHOSEN_SUB_EMOTE_UNLOCK: { title: 'Unlock a sub emote', needsText: false, emote: 'unlock' },
    CHOSEN_MODIFIED_SUB_EMOTE_UNLOCK: { title: 'Modify an emote', needsText: false, emote: 'modify' },
  };

  let open = $state(false);
  let points = $state<Points | null>(null);
  let loaded = $state(false);
  let picked = $state<Choice | null>(null);
  let text = $state('');
  let emote = $state<string | null>(null);
  let busy = $state(false);
  let page = $state<'points' | 'bits'>('points');

  const canRedeem = $derived(room.account !== null && accounts.can('points', room.account));
  const name = $derived(points?.name ?? 'Channel points');

  const choices = $derived.by((): Choice[] => {
    if (!points) return [];
    const auto = points.automatic
      .filter((r) => AUTOMATIC[r.kind])
      .map((r) => ({
        id: r.id,
        title: AUTOMATIC[r.kind].title,
        cost: r.cost,
        color: r.color,
        image: r.image,
        prompt: null,
        needsText: AUTOMATIC[r.kind].needsText,
        emote: AUTOMATIC[r.kind].emote,
        available: r.available,
      }));
    const custom = points.custom.map((r) => ({
      id: r.id,
      title: r.title,
      cost: r.cost,
      color: r.color,
      image: r.image,
      prompt: r.prompt,
      needsText: r.inputRequired,
      available: r.available,
    }));
    return [...auto, ...custom].sort((a, b) => a.cost - b.cost);
  });

  // V101-13 The channel's own points icon, and the balance with a GQL sign-in, show
  // next to the box once the chat is open, from what joining already loaded.
  $effect(() => {
    if (room.view && !loaded) void load(false);
  });

  $effect(() => {
    const off = minerIpc.onBalance((b) => {
      if (points && b.account === room.account && b.channelId === room.state.channelId) {
        points = { ...points, balance: b.balance };
      }
    });
    return () => void off.then((f) => f());
  });

  /** `fresh`: rewards as they are now, not as they were when the chat opened. */
  async function load(fresh = true) {
    loaded = true;
    try {
      points = await chatIpc.points(room.view, fresh);
    } catch {
      if (fresh) points = null;
    }
  }

  function toggle() {
    open = !open;
    picked = null;
    page = 'points';
    if (open) void load();
  }

  /** Every reward asks once more before spending points. */
  function pick(c: Choice) {
    if (!canRedeem || !c.available) return;
    picked = c;
    text = '';
    emote = null;
  }

  async function redeem() {
    if (!picked || busy) return;
    busy = true;
    try {
      toast.info(
        await chatIpc.redeem(room.view, picked.id, picked.needsText ? text : undefined, emote ?? undefined),
      );
      open = false;
      picked = null;
      void load();
    } catch (e) {
      toast.error(String(e));
    } finally {
      busy = false;
    }
  }
</script>

<div class="points">
  <button class="trigger" onclick={toggle} use:tooltip={{ label: name, hint: 'Rewards you can redeem here.' }}>
    {#if points?.icon}<img src={points.icon} alt="" />{:else}<Icon name="points" size={16} />{/if}
    {#if points?.balance != null}<span>{formatShort(points.balance)}</span>{/if}
  </button>
  <Popover bind:open align="start" side="above">
    <div class="menu">
      <header>
        {#if canRedeem}
          <Segmented
            label="Pay with"
            options={[
              { value: 'points', label: name },
              { value: 'bits', label: 'Power-ups' },
            ]}
            value={page}
            onchange={(v) => (page = v)}
          />
        {:else}
          <strong>{name}</strong>
        {/if}
        {#if points?.balance != null && page === 'points'}<span class="balance">{formatNumber(points.balance)}</span>{/if}
      </header>
      {#if !canRedeem}
        <!-- V102-20 -->
        <p class="note locked">Redeeming needs a GQL sign-in for this account. <GateFlag kind="gql" /></p>
      {/if}
      {#if page === 'bits'}
        <PowerUps {room} ondone={() => (open = false)} />
      {:else if picked}
        <div class="confirm">
          <div class="title">{picked.title} · {formatNumber(picked.cost)}</div>
          {#if picked.prompt}<p class="note">{picked.prompt}</p>{/if}
          {#if picked.emote}
            <EmotePick view={room.view} kind={picked.emote} bind:picked={emote} />
          {/if}
          {#if picked.needsText}
            <!-- svelte-ignore a11y_autofocus -->
            <input
              class="field"
              bind:value={text}
              maxlength="500"
              placeholder="Your message"
              autofocus
              onkeydown={(e) => e.key === 'Enter' && void redeem()}
            />
          {/if}
          <div class="actions">
            <Button small variant="ghost" onclick={() => (picked = null)}>Back</Button>
            <Button
              small
              variant="primary"
              disabled={busy || (picked.needsText && !text.trim()) || (!!picked.emote && !emote)}
              onclick={redeem}
            >
              Redeem
            </Button>
          </div>
        </div>
      {:else if !loaded}
        <p class="note">Loading…</p>
      {:else if !choices.length}
        <p class="note">No rewards here.</p>
      {:else}
        <div class="grid">
          {#each choices as c (c.id)}
            <button
              class="reward"
              disabled={!canRedeem || !c.available}
              style:--rc={c.color}
              onclick={() => pick(c)}
              use:tooltip={{ label: c.title, hint: c.prompt ?? undefined }}
            >
              <span class="tile">{#if c.image}<img src={c.image} alt="" />{/if}</span>
              <span class="rt">{c.title}</span>
              <span class="cost">{formatNumber(c.cost)}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </Popover>
</div>

<style>
  .points {
    position: relative;
  }
  .trigger {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 34px;
    padding: 0 8px;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.8rem;
    font-weight: 700;
  }
  .trigger:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .trigger img {
    width: 16px;
    height: 16px;
  }
  .menu {
    width: 320px;
    padding: 6px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }
  .balance {
    font-weight: 700;
    color: var(--twitch);
  }
  .note {
    margin: 4px 0 8px;
    color: var(--dim);
    font-size: 0.8rem;
  }
  .note.locked {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 6px;
    max-height: 300px;
    overflow-y: auto;
  }
  .reward {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 6px 4px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    font-size: 0.75rem;
  }
  .reward:hover:not(:disabled) {
    border-color: var(--rc);
  }
  .reward:disabled {
    opacity: 0.45;
  }
  .tile {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    background: var(--rc);
    border-radius: var(--radius);
  }
  .tile img {
    width: 28px;
    height: 28px;
  }
  .rt {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    text-align: center;
  }
  .cost {
    color: var(--dim);
    font-weight: 700;
  }
  .confirm .title {
    font-weight: 700;
    margin-bottom: 6px;
  }
  .confirm .field {
    width: 100%;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    margin-top: 8px;
  }
</style>
