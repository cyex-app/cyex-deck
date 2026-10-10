<!-- 1.42 Power-ups paid with Bits: message effects, a gigantified emote, a celebration,
     and the channel's Bits rewards. Spending always asks once more, with the price. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import type { PowerUpBuy } from '../../ipc/bindings/PowerUpBuy';
  import type { PowerUps } from '../../ipc/bindings/PowerUps';
  import { chatxIpc } from '../../ipc/chatx';
  import { formatNumber } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import { usable } from '../emotes/menu';
  import { emoteMenu } from '../emotes/menus';
  import type { ChatRoom } from './room.svelte';

  let { room, ondone }: { room: ChatRoom; ondone: () => void } = $props();

  type Kind = 'effect' | 'gigantify' | 'celebration' | 'reward';
  interface Pick {
    kind: Kind;
    title: string;
    bits: number;
    /** Reward id, or the effect's id. */
    id?: string;
    needsText: boolean;
  }

  const EFFECTS = [
    { id: 'simmer', title: 'Simmer' },
    { id: 'rainbow-eclipse', title: 'Rainbow Eclipse' },
    { id: 'cosmic-abyss', title: 'Cosmic Abyss' },
  ];

  let ups = $state<PowerUps | null>(null);
  let error = $state('');
  let picked = $state<Pick | null>(null);
  let text = $state('');
  let sure = $state(false);
  let busy = $state(false);
  /** Twitch emotes you can use here, by name. */
  let twitch = $state.raw(new Map<string, string>());

  // svelte-ignore state_referenced_locally
  chatxIpc
    .powerUps(room.view)
    .then((u) => (ups = u))
    .catch((e) => (error = String(e)));
  // svelte-ignore state_referenced_locally
  emoteMenu(room.login, room.account)
    .then((m) => {
      twitch = new Map([...usable(m).values()].filter((e) => e.provider === 'twitch').map((e) => [e.name, e.id]));
    })
    .catch(() => {});

  const lastWord = $derived(text.trim().split(/\s+/).at(-1) ?? '');

  /** What's wrong with the input, if anything. */
  const problem = $derived.by(() => {
    if (!picked) return '';
    if (picked.needsText && !text.trim()) return 'Write a message.';
    if (picked.kind === 'gigantify' && !twitch.has(lastWord)) return 'End the message with a Twitch emote you can use.';
    if (picked.kind === 'celebration' && !twitch.has(text.trim())) return 'Type one Twitch emote you can use.';
    if (ups?.balance != null && ups.balance < picked.bits) return 'Not enough Bits.';
    return '';
  });

  function choose(p: Pick) {
    picked = p;
    text = '';
    sure = false;
  }

  function order(p: Pick): PowerUpBuy {
    switch (p.kind) {
      case 'effect':
        return { kind: 'effect', effect: p.id!, message: text.trim() };
      case 'gigantify':
        return { kind: 'gigantify', emote_id: twitch.get(lastWord)!, message: text.trim() };
      case 'celebration':
        return { kind: 'celebration', emote_id: twitch.get(text.trim())! };
      case 'reward':
        return { kind: 'reward', id: p.id!, text: text.trim() };
    }
  }

  async function buy() {
    if (!picked || problem || busy) return;
    if (!sure) return void (sure = true);
    busy = true;
    try {
      await chatxIpc.buyPowerUp(room.view, order(picked), picked.bits);
      toast.info('Sent');
      ondone();
    } catch (e) {
      toast.error(String(e));
      sure = false;
    } finally {
      busy = false;
    }
  }

  /** On, off cooldown, and affordable when the balance is known. */
  const ready = (p: { available: boolean; cooldownUntil: string | null; bits: number } | null) =>
    !!p &&
    p.available &&
    (!p.cooldownUntil || Date.parse(p.cooldownUntil) < Date.now()) &&
    (ups?.balance == null || ups.balance >= p.bits);
</script>

{#if error}
  <p class="note">{error}</p>
{:else if !ups}
  <p class="note">Loading…</p>
{:else if picked}
  <div class="confirm">
    <div class="title">{picked.title} · {formatNumber(picked.bits)} Bits</div>
    {#if picked.needsText || picked.kind === 'celebration'}
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="field"
        bind:value={text}
        maxlength="500"
        placeholder={picked.kind === 'celebration' ? 'Emote name' : 'Your message'}
        autofocus
        onkeydown={(e) => e.key === 'Enter' && void buy()}
      />
    {/if}
    {#if problem && text}<p class="note">{problem}</p>{/if}
    {#if sure}<p class="ask">Spend {formatNumber(picked.bits)} Bits?</p>{/if}
    <div class="actions">
      <Button small variant="ghost" onclick={() => (sure ? (sure = false) : (picked = null))}>Back</Button>
      <Button small variant="primary" disabled={busy || !!problem} onclick={buy}>
        {sure ? 'Spend' : 'Next'}
      </Button>
    </div>
  </div>
{:else}
  {@const fx = ups.effect}
  {@const giant = ups.gigantify}
  {@const party = ups.celebration}
  {#if ups.balance != null}<p class="note">You have {formatNumber(ups.balance)} Bits.</p>{/if}
  <div class="list">
    {#if fx}
      {#each EFFECTS as e (e.id)}
        <button
          disabled={!ready(fx)}
          onclick={() => choose({ kind: 'effect', id: e.id, title: e.title, bits: fx.bits, needsText: true })}
        >
          <span>{e.title}</span><span class="cost">{formatNumber(fx.bits)}</span>
        </button>
      {/each}
    {/if}
    {#if giant}
      <button
        disabled={!ready(giant)}
        onclick={() => choose({ kind: 'gigantify', title: 'Gigantify an emote', bits: giant.bits, needsText: true })}
      >
        <span>Gigantify an emote</span><span class="cost">{formatNumber(giant.bits)}</span>
      </button>
    {/if}
    {#if party}
      <button
        disabled={!ready(party)}
        onclick={() => choose({ kind: 'celebration', title: 'Celebration', bits: party.bits, needsText: false })}
      >
        <span>Celebration</span><span class="cost">{formatNumber(party.bits)}</span>
      </button>
    {/if}
    {#each ups.custom as r (r.id)}
      <button
        disabled={!ready(r)}
        onclick={() => choose({ kind: 'reward', id: r.id, title: r.title, bits: r.bits, needsText: r.needsText })}
      >
        <span>{r.title}</span><span class="cost">{formatNumber(r.bits)}</span>
      </button>
    {/each}
    {#if !fx && !giant && !party && !ups.custom.length}<p class="note">No power-ups here.</p>{/if}
  </div>
{/if}

<style>
  .note {
    margin: 4px 0 8px;
    color: var(--dim);
    font-size: 0.8rem;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 300px;
    overflow-y: auto;
  }
  .list button {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 8px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    font-size: 0.8rem;
    text-align: left;
  }
  .list button:hover:not(:disabled) {
    border-color: var(--twitch);
  }
  .list button:disabled {
    opacity: 0.45;
  }
  .cost {
    color: var(--twitch);
    font-weight: 700;
    white-space: nowrap;
  }
  .title {
    font-weight: 700;
    margin-bottom: 6px;
  }
  .field {
    width: 100%;
  }
  .ask {
    margin: 8px 0 0;
    font-weight: 600;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    margin-top: 8px;
  }
</style>
