<!-- V107-B Bits picker next to the rewards: the channel's cheermotes and their tiers, an
     amount and your balance. A pick puts "PrefixN" in the box; Enter sends it through the
     cheer path. Tiers you can't afford are greyed. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import Loading from '../../components/Loading.svelte';
  import NumberField from '../../components/NumberField.svelte';
  import Popover from '../../components/Popover.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { Cheermote } from '../../ipc/bindings/Cheermote';
  import { density } from '../../lib/density.svelte';
  import { formatNumber } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import type { CheerBox } from './bits.svelte';
  import { catalogs } from './catalog.svelte';
  import type { ChatRoom } from './room.svelte';
  import { cheerImg, cheerTier } from './urls';

  interface Props {
    room: ChatRoom;
    box: CheerBox;
    oninsert: (prefix: string, amount: number) => void;
  }

  let { room, box, oninsert }: Props = $props();

  const MAX = 100_000;
  let open = $state(false);
  let amount = $state(100);
  const theme = $derived(settings.v.appearance.theme);
  const info = $derived(box.info);
  const balance = $derived(info?.balance ?? null);
  const list = $derived.by((): Cheermote[] => {
    const usable = new Set(info?.prefixes ?? []);
    return catalogs.cheermotes(room.login).filter((c) => usable.has(c.prefix.toLowerCase()));
  });
  const afford = (bits: number) => balance === null || bits <= balance;

  function toggle() {
    open = !open;
    if (!open) return;
    void catalogs.loadGlobal();
    void catalogs.loadChannel(room.login);
    void box.load();
  }

  function pick(c: Cheermote, bits: number) {
    oninsert(c.prefix, bits);
    open = false;
  }
</script>

<div class="bits">
  <button
    class="trigger"
    aria-label="Cheer"
    aria-haspopup="dialog"
    aria-expanded={open}
    onclick={toggle}
    use:tooltip={{
      label: balance != null ? `Cheer · ${formatNumber(balance)} Bits` : 'Cheer',
      hint: 'Pick a cheer, then send it with Enter.',
    }}
  >
    <Icon name="gem" size={16} />
  </button>
  <Popover bind:open align="start" side="above" label="Cheer">
    <div class="menu">
      <header>
        <strong>Cheer</strong>
        {#if balance != null}<span class="balance">{formatNumber(balance)} Bits</span>{/if}
      </header>
      {#if box.error && !info}
        <p class="note">{box.error}</p>
      {:else if !info}
        <Loading />
      {:else if !info.enabled}
        <p class="note">Cheering is off in this channel.</p>
      {:else}
        <div class="amount">
          <span>Amount</span>
          <NumberField label="Amount" value={amount} min={1} max={MAX} width="96px" onchange={(v) => (amount = v)} />
          {#if info.cheerMin > 1}<span class="min">Minimum {formatNumber(info.cheerMin)}</span>{/if}
        </div>
        {#if !list.length}
          <p class="note">No cheers here yet.</p>
        {:else}
          <div class="list">
            {#each list as c (c.prefix)}
              {@const tier = cheerTier(c, amount)}
              <div class="cm">
                <button class="use" disabled={!afford(amount)} onclick={() => pick(c, amount)} title="{c.prefix}{amount}">
                  {#if tier}<img class="pale-edge" src={cheerImg(c, tier, theme, density.scale).src} alt="" loading="lazy" />{/if}
                  <span class="name">{c.prefix}</span>
                  <span class="n" style:color={tier?.color}>{formatNumber(amount)}</span>
                </button>
                <div class="tiers">
                  {#each c.tiers as t (t.bits)}
                    <button
                      class="tier"
                      disabled={!afford(t.bits)}
                      onclick={() => pick(c, t.bits)}
                      aria-label="{c.prefix}{t.bits}"
                    >
                      <img class="pale-edge" src={cheerImg(c, t, theme, density.scale).src} alt="" loading="lazy" />
                      <span style:color={t.color}>{formatNumber(t.bits)}</span>
                    </button>
                  {/each}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      {/if}
    </div>
  </Popover>
</div>

<style>
  .bits {
    position: relative;
  }
  .trigger {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    padding: 0;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius);
    color: var(--muted);
  }
  .trigger:hover,
  .trigger[aria-expanded='true'] {
    background: var(--surface-hover);
    color: var(--text);
  }
  .menu {
    width: 340px;
    max-width: calc(100vw - 24px);
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
    font-variant-numeric: tabular-nums;
  }
  .note {
    margin: 4px 0 8px;
    color: var(--dim);
    font-size: 0.8rem;
  }
  .amount {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
    font-size: 0.8rem;
    color: var(--muted);
  }
  .min {
    color: var(--dim);
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 300px;
    overflow-y: auto;
  }
  .cm {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 4px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .use {
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 28px;
    padding: 2px 4px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    font-size: 0.8rem;
    text-align: left;
  }
  .use:hover:not(:disabled),
  .tier:hover:not(:disabled) {
    background: var(--surface-hover);
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }
  .n,
  .tier span {
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  /* Twitch's tier colors are made for dark: the yellow and light blue fade on white. */
  :global(:root[data-theme='light']) .n,
  :global(:root[data-theme='light']) .tier span {
    filter: brightness(0.75);
  }
  .tiers {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
  }
  .tier {
    display: flex;
    align-items: center;
    gap: 4px;
    min-height: 24px;
    padding: 2px 6px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    font-size: 0.72rem;
  }
  img {
    width: 20px;
    height: 20px;
    flex: none;
  }
  .tier img {
    width: 16px;
    height: 16px;
  }
  button:disabled {
    opacity: 0.4;
  }
</style>
