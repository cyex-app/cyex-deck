<!-- 1.40 How you show up in chat: the badge you wear everywhere or just here, and your
     name color. Badges need a GQL sign-in; anyone picks from Twitch's named colors,
     Prime and Turbo pick any. -->
<script lang="ts">
  import { onMount } from 'svelte';

  import Button from '../../components/Button.svelte';
  import Dialog from '../../components/Dialog.svelte';
  import type { BadgeInfo } from '../../ipc/bindings/BadgeInfo';
  import type { ChatIdentity } from '../../ipc/bindings/ChatIdentity';
  import { chatIpc } from '../../ipc/chat';
  import { toast } from '../../lib/toast.svelte';

  interface Props {
    account: string;
    login: string;
    onclose: () => void;
  }

  let { account, login, onclose }: Props = $props();

  const NAMED: [string, string][] = [
    ['Blue', '#0000FF'],
    ['Blue violet', '#8A2BE2'],
    ['Cadet blue', '#5F9EA0'],
    ['Chocolate', '#D2691E'],
    ['Coral', '#FF7F50'],
    ['Dodger blue', '#1E90FF'],
    ['Firebrick', '#B22222'],
    ['Golden rod', '#DAA520'],
    ['Green', '#008000'],
    ['Hot pink', '#FF69B4'],
    ['Orange red', '#FF4500'],
    ['Red', '#FF0000'],
    ['Sea green', '#2E8B57'],
    ['Spring green', '#00FF7F'],
    ['Yellow green', '#9ACD32'],
  ];

  let me = $state<ChatIdentity | null>(null);
  let error = $state('');
  let busy = $state(false);
  let custom = $state('#');

  async function load() {
    try {
      me = await chatIpc.identity(account, login);
      custom = me.color ?? '#';
    } catch (e) {
      error = String(e);
    }
  }

  onMount(() => void load());

  async function run(action: () => Promise<void>, done: string) {
    if (busy) return;
    busy = true;
    try {
      await action();
      toast.info(done);
      await load();
    } catch (e) {
      toast.error(String(e));
    } finally {
      busy = false;
    }
  }

  const color = (hex: string) => run(() => chatIpc.setColor(account, hex), 'Color changed.');
  const wear = (channel: boolean, b: BadgeInfo | null) =>
    run(
      () => chatIpc.pickBadge(account, channel ? (me?.channelId ?? null) : null, b?.set ?? null, b?.version ?? null),
      b ? `Wearing ${b.title}.` : 'Badge off.',
    );
  const same = (a: BadgeInfo | null, b: BadgeInfo | null) => a?.set === b?.set && a?.version === b?.version;
</script>

{#snippet picks(channel: boolean, selected: BadgeInfo | null, available: BadgeInfo[])}
  <div class="badges">
    <button class="badge none" class:on={!selected} disabled={busy} onclick={() => wear(channel, null)}>
      {channel ? 'Same as everywhere' : 'None'}
    </button>
    {#each available as b (`${b.set}/${b.version}`)}
      <button
        class="badge"
        class:on={same(selected, b)}
        disabled={busy}
        title={b.title}
        aria-label={b.title}
        onclick={() => wear(channel, b)}
      >
        <img src={b.image2x} alt="" />
      </button>
    {/each}
  </div>
{/snippet}

<Dialog title="Badge and color" width={420} {onclose}>
  {#if error}
    <p class="note">{error}</p>
  {:else if !me}
    <p class="note">Loading…</p>
  {:else}
    <h3 class="label">Name color</h3>
    <div class="colors">
      {#each NAMED as [name, hex] (hex)}
        <button
          class="swatch"
          class:on={me.color?.toUpperCase() === hex}
          style:background={hex}
          title={name}
          aria-label={name}
          disabled={busy}
          onclick={() => color(hex)}
        ></button>
      {/each}
    </div>
    {#if me.anyColor}
      <div class="custom">
        <input class="field" bind:value={custom} maxlength="7" spellcheck="false" aria-label="Any color" />
        <input type="color" bind:value={custom} aria-label="Pick a color" />
        <Button small disabled={busy || !/^#[0-9a-fA-F]{6}$/.test(custom)} onclick={() => color(custom)}>Use</Button>
      </div>
    {:else}
      <p class="note">Prime or Turbo can pick any color.</p>
    {/if}

    {#if me.global.available.length || me.channel.available.length}
      <h3 class="label">Badge everywhere</h3>
      {@render picks(false, me.global.selected, me.global.available)}
      {#if me.channel.available.length && me.channelId}
        <h3 class="label">Badge in {login}'s chat</h3>
        {@render picks(true, me.channel.selected, me.channel.available)}
      {/if}
    {:else if me.channelId}
      <p class="note">No badges to pick from.</p>
    {:else}
      <p class="note">Picking a badge needs a GQL sign-in.</p>
    {/if}
  {/if}
</Dialog>

<style>
  h3 {
    margin: 4px 0 8px;
  }
  h3 + * {
    margin-bottom: 16px;
  }
  .colors {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .swatch {
    width: 26px;
    height: 26px;
    padding: 0;
    border: 2px solid var(--border-strong);
    border-radius: 50%;
  }
  .swatch.on {
    outline: 2px solid var(--accent-border);
    outline-offset: 2px;
  }
  .custom {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .custom .field {
    width: 100px;
    font-family: var(--font-mono);
  }
  .custom input[type='color'] {
    width: 34px;
    height: 30px;
    padding: 0;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: none;
  }
  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .badge {
    display: grid;
    place-items: center;
    min-width: 40px;
    height: 40px;
    padding: 4px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
  }
  .badge img {
    width: 28px;
    height: 28px;
  }
  .badge.none {
    padding: 0 10px;
    font-size: 0.8rem;
    color: var(--muted);
  }
  .badge.on {
    border-color: var(--accent-border);
    background: var(--accent-dim);
  }
  .note {
    margin: 4px 0 12px;
    color: var(--dim);
    font-size: 0.85rem;
  }
</style>
