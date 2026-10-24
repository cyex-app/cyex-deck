<!-- V107-B Bits picker next to the rewards. V107-H10a Amount first (presets or any number),
     then the style: one icon per cheermote at the tier the amount reaches. A pick puts
     "PrefixN" in the box; Enter sends it through the cheer path. -->
<script lang="ts">
  import { tick } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import Loading from '../../components/Loading.svelte';
  import NumberField from '../../components/NumberField.svelte';
  import Popover from '../../components/Popover.svelte';
  import { moveFor } from '../../components/select.ts';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { Cheermote } from '../../ipc/bindings/Cheermote';
  import { density } from '../../lib/density.svelte';
  import { formatNumber } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import type { CheerBox } from './bits.svelte';
  import { catalogs } from './catalog.svelte';
  import { CHEER_MAX, canAfford, cheerFloor, cheerPresets, startAmount, wordMin, type CheerRules } from './cheer';
  import type { ChatRoom } from './room.svelte';
  import { cheerImg, cheerTier } from './urls';

  interface Props {
    room: ChatRoom;
    box: CheerBox;
    oninsert: (prefix: string, amount: number) => void;
  }

  let { room, box, oninsert }: Props = $props();

  let open = $state(false);
  let amount = $state(100);
  const theme = $derived(settings.v.appearance.theme);
  const info = $derived(box.info);
  const balance = $derived(info?.balance ?? null);
  const rules = $derived<CheerRules>({ cheerMin: info?.cheerMin ?? 1, emoteMin: info?.emoteMin ?? 1, balance });
  const presets = $derived(cheerPresets(rules));
  // Under the channel's minimum a style can't go out, same as an amount over the balance.
  const ok = $derived(canAfford(amount, rules) && amount >= cheerFloor(rules));
  const list = $derived.by((): Cheermote[] => {
    const usable = new Set(info?.prefixes ?? []);
    return catalogs.cheermotes(room.login).filter((c) => usable.has(c.prefix.toLowerCase()));
  });
  // V107-H1b The amount starts at the channel's minimum when that's higher.
  let lifted = '';
  $effect(() => {
    if (!info || lifted === room.login) return;
    lifted = room.login;
    amount = startAmount(amount, rules);
  });

  let trigger = $state<HTMLButtonElement>();
  let wasOpen = false;
  let focused = false;
  // Roving tab stops: the last chip / style that had focus (null: the default one).
  let chipAt = $state<number | null>(null);
  let styleAt = $state<string | null>(null);
  const chipStop = $derived.by(() => {
    const at = presets.find((p) => p.ok && p.bits === chipAt) ?? presets.find((p) => p.ok && p.bits === amount) ?? presets.find((p) => p.ok);
    return at?.bits ?? null;
  });
  const styleStop = $derived(ok ? (list.find((c) => c.prefix === styleAt) ?? list[0])?.prefix : undefined);

  // Opened: focus lands on the first chip once the rules are in. Closed: back on the gem,
  // or on the "…" button when the gem sits folded away.
  $effect(() => {
    if (open) {
      wasOpen = true;
      if (info?.enabled && !focused) {
        focused = true;
        void tick().then(() => document.querySelector<HTMLElement>('.bits-menu .chip:not(:disabled)[tabindex="0"]')?.focus());
      }
      return;
    }
    focused = false;
    if (!wasOpen) return;
    wasOpen = false;
    void tick().then(() => {
      const at = document.activeElement;
      if (at && at !== document.body && !at.matches('[role="dialog"], [tabindex="-1"]')) return;
      const t = trigger;
      if (t?.getClientRects().length) t.focus();
      else t?.closest<HTMLElement>('.row')?.querySelector<HTMLElement>('button.more')?.focus();
    });
  });

  function toggle() {
    open = !open;
    if (!open) return;
    void catalogs.loadGlobal();
    void catalogs.loadChannel(room.login);
    void box.load();
  }

  function pick(c: Cheermote) {
    if (!canAfford(amount, rules)) return;
    oninsert(c.prefix, amount);
    open = false;
  }

  /** Arrows move inside a row of choices; Tab leaves it. */
  function rove(e: KeyboardEvent) {
    const key = e.key === 'ArrowLeft' ? 'ArrowUp' : e.key === 'ArrowRight' ? 'ArrowDown' : e.key;
    if (!['ArrowUp', 'ArrowDown', 'Home', 'End'].includes(key)) return;
    const row = [...(e.currentTarget as HTMLElement).querySelectorAll<HTMLButtonElement>('button:not(:disabled)')];
    const to = moveFor(key, row.indexOf(document.activeElement as HTMLButtonElement), row.length);
    if (to === null || to < 0) return;
    e.preventDefault();
    row[to].focus();
  }

  function track(e: FocusEvent) {
    const t = e.target as HTMLElement;
    if (t.dataset.bits) chipAt = Number(t.dataset.bits);
    else if (t.dataset.prefix) styleAt = t.dataset.prefix;
  }
</script>

<div class="bits">
  <button
    class="trigger"
    bind:this={trigger}
    aria-label="Cheer"
    aria-haspopup="dialog"
    aria-expanded={open}
    onclick={toggle}
    use:tooltip={{
      label: balance != null ? `Cheer · ${formatNumber(balance)} Bits` : 'Cheer',
      hint: 'Pick an amount and a cheer, then send it with Enter.',
    }}
  >
    <Icon name="gem" size={16} />
  </button>
  <Popover bind:open align="start" side="above" label="Cheer">
    <div class="menu bits-menu">
      <header>
        <strong>Cheer</strong>
        {#if balance != null}<span class="balance">{formatNumber(balance)} Bits</span>{/if}
      </header>
      {#if box.error && !info}
        <p class="note">{box.error}</p>
      {:else if !info}
        <Loading />
      {:else}
        {#if box.error}<p class="note">Balance may be out of date.</p>{/if}
        <div class="row" role="toolbar" tabindex="-1" aria-label="Amount" onkeydown={rove} onfocusin={track}>
          {#each presets as p (p.bits)}
            <button class="chip" data-bits={p.bits} tabindex={p.bits === chipStop ? 0 : -1} aria-pressed={amount === p.bits} disabled={!p.ok} onclick={() => (amount = p.bits)}>
              {formatNumber(p.bits)}
            </button>
          {/each}
        </div>
        <div class="amount">
          <NumberField label="Amount" value={amount} min={wordMin(rules)} max={CHEER_MAX} width="96px" unit="Bits" onchange={(v) => (amount = v)} />
          {#if balance != null && amount > balance}
            <span class="warn">More than you have</span>
          {:else if info.cheerMin > 1}
            <span class="min">Minimum {formatNumber(info.cheerMin)}</span>
          {/if}
        </div>
        {#if !list.length}
          <p class="note">No cheers here yet.</p>
        {:else}
          <div class="styles" role="toolbar" tabindex="-1" aria-label="Style" onkeydown={rove} onfocusin={track}>
            {#each list as c (c.prefix)}
              {@const tier = cheerTier(c, amount)}
              <button
                class="style"
                data-prefix={c.prefix}
                tabindex={c.prefix === styleStop ? 0 : -1}
                disabled={!ok}
                aria-label="{c.prefix}{amount}"
                onclick={() => pick(c)}
                use:tooltip={{ label: `${c.prefix}${formatNumber(amount)}` }}
              >
                {#if tier}<img class="pale-edge" src={cheerImg(c, tier, theme, density.scale).src} alt="" loading="lazy" />{/if}
              </button>
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
    width: 30px;
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
  @container chat (max-width: 260px) {
    .trigger {
      width: 28px;
    }
  }
  .menu {
    width: 300px;
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
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-bottom: 6px;
  }
  .chip {
    min-width: 44px;
    height: 32px;
    padding: 0 8px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    font-size: var(--text-hint);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .chip:hover:not(:disabled),
  .style:hover:not(:disabled) {
    background: var(--surface-hover);
  }
  .chip[aria-pressed='true'] {
    background: var(--accent-wash);
    border-color: var(--accent-border);
  }
  .amount {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
    flex-wrap: wrap;
    font-size: var(--text-hint);
  }
  .min {
    color: var(--dim);
  }
  .warn {
    color: var(--red);
  }
  .styles {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(40px, 1fr));
    gap: 4px;
    max-height: 200px;
    overflow-y: auto;
  }
  .style {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 40px;
    padding: 0;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius);
  }
  img {
    width: 28px;
    height: 28px;
  }
  /* Dim with colour, not opacity: the text stays readable. */
  .chip:disabled {
    color: var(--dim);
  }
  .style:disabled img {
    opacity: 0.4;
  }
</style>
