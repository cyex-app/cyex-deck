<!-- 1.40 V107-Mk Chat identity, as on twitch.tv: the badge you wear (everywhere, or with
     V107-Mx's switch one just for this channel: any badge you have, V107-H11a), your name color, and
     with Prime or Turbo any color and a smiley style. Every change is read back from Twitch.
     Shown in the chat box's popover and the account chip's dialog. -->
<script lang="ts">
  import { untrack } from 'svelte';

  import Button from '../../components/Button.svelte';
  import Loading from '../../components/Loading.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { BadgeInfo } from '../../ipc/bindings/BadgeInfo';
  import type { ChatIdentity } from '../../ipc/bindings/ChatIdentity';
  import type { SmileyChoice } from '../../ipc/bindings/SmileyChoice';
  import { chatIpc } from '../../ipc/chat';
  import { accounts } from '../../lib/accounts.svelte';
  import { density } from '../../lib/density.svelte';
  import { errorText } from '../../lib/errors';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { hexColor, NAMED, namedColor, STYLE_NAME } from './identity';
  import { emoteImg } from './urls';

  interface Props {
    account: string;
    login: string;
  }

  let { account, login }: Props = $props();

  let me = $state<ChatIdentity | null>(null);
  let error = $state('');
  let busy = $state(false);
  let custom = $state('');
  /** V107-Mx Switched on, nothing worn here yet: the list shows, Twitch is told on a pick. */
  let ownWanted = $state(false);

  const theme = $derived(settings.v.appearance.theme);
  const display = $derived(accounts.list.find((a) => a.userId === account)?.displayName ?? '');
  const shown = $derived(me?.channel.selected ?? me?.global.selected ?? null);
  const styles = $derived(me?.smilies.choices ?? []);
  const typed = $derived(hexColor(custom));
  const same = (a: BadgeInfo | null, b: BadgeInfo | null) => a?.set === b?.set && a?.version === b?.version;
  // A channel pick equal to the global one counts as none: the read may repeat the global badge.
  const ownPick = $derived(me?.channel.selected && !same(me.channel.selected, me.global.selected) ? me.channel.selected : null);
  const own = $derived(!!ownPick || ownWanted);

  // Only the newest read lands: switching account or channel mid-read drops the old one.
  let reads = 0;
  async function load() {
    const n = ++reads;
    try {
      const got = await chatIpc.identity(account, login);
      if (n !== reads) return;
      me = got;
      custom = got.color ?? '';
      error = '';
    } catch (e) {
      if (n === reads) error = errorText(e);
    }
  }

  // Read again for another account or channel, so picks never go to the wrong one.
  $effect(() => {
    void account;
    void login;
    me = null;
    error = '';
    ownWanted = false;
    void untrack(load);
  });

  /** `reload` false: the action already answered with what Twitch has now. */
  async function run(action: () => Promise<unknown>, done: string, reload = true) {
    if (busy) return;
    busy = true;
    try {
      await action();
      if (reload) await load();
      toast.info(done);
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = false;
    }
  }

  const color = (hex: string) => run(() => chatIpc.setColor(account, hex), 'Name color changed.');
  const wear = (channel: boolean, b: BadgeInfo | null) =>
    run(
      () => chatIpc.pickBadge(account, channel ? (me?.channelId ?? null) : null, b?.set ?? null, b?.version ?? null),
      b ? `Wearing ${b.title}.` : channel ? 'Same badge as everywhere.' : 'Badge off.',
    );
  /** Off takes the channel badge off, back to the one worn everywhere. */
  const switchOwn = (on: boolean) => {
    ownWanted = on;
    if (!on && ownPick) void wear(true, null);
  };
  const smile = (c: SmileyChoice) => {
    const [who, at] = [account, login];
    return run(
      async () => {
        const now = await chatIpc.setSmilies(who, c.name);
        if (me && who === account && at === login) me.smilies = now;
      },
      `${STYLE_NAME[c.style]} smileys on.`,
      false,
    );
  };
  const img = (id: string) => emoteImg('twitch', id, theme, { scale: density.scale }).src;
</script>

{#snippet picks(channel: boolean, selected: BadgeInfo | null, available: BadgeInfo[])}
  <div class="badges">
    {#if !channel}
      <button class="badge none" class:on={!selected} aria-pressed={!selected} disabled={busy} use:tooltip={'No badge'} onclick={() => wear(false, null)}>
        None
      </button>
    {/if}
    {#each available as b (`${b.set}/${b.version}`)}
      <button
        class="badge"
        class:on={same(selected, b)}
        aria-pressed={same(selected, b)}
        disabled={busy}
        use:tooltip={b.title}
        aria-label={b.title}
        onclick={() => wear(channel, b)}
      >
        <img class="pale-edge" src={b.image2x} alt="" />
      </button>
    {/each}
  </div>
{/snippet}

<div class="panel" aria-busy={busy}>
  {#if error}
    <p class="note">{error}</p>
  {:else if !me}
    <Loading />
  {:else}
    <div class="preview" aria-hidden="true">
      {#if shown}<img class="pale-edge" src={shown.image2x} alt="" />{/if}
      <b style:color={me.color ?? 'var(--text)'}>{display}</b>
    </div>

    <section>
      <h3 class="label">Badge</h3>
      {#if me.channelId}
        {#if me.global.available.length}
          <p class="sub">Everywhere</p>
          {@render picks(false, me.global.selected, me.global.available)}
        {/if}
        <!-- V107-H11a Always there with a GQL sign-in, as on twitch.tv; the grid lists every
             badge you can wear here, the global ones too. -->
        <div class="own">
          <span>
            Custom badge for this channel
            <small>A different badge, only in {login}’s chat.</small>
          </span>
          <Toggle checked={own} label="Custom badge for {login}" disabled={busy} onchange={switchOwn} />
        </div>
        {#if own}
          {#if me.channel.available.length}
            {@render picks(true, ownPick, me.channel.available)}
          {:else}
            <p class="note">No badges to wear here.</p>
          {/if}
        {/if}
      {:else}
        <p class="note">Picking a badge needs a GQL sign-in.</p>
      {/if}
    </section>

    <section>
      <h3 class="label">Name color</h3>
      <div class="colors">
        {#each NAMED as [name, hex] (hex)}
          <button
            class="swatch"
            class:on={me.color?.toUpperCase() === hex}
            aria-pressed={me.color?.toUpperCase() === hex}
            style:background={hex}
            use:tooltip={name}
            aria-label={name}
            disabled={busy}
            onclick={() => color(hex)}
          ></button>
        {/each}
      </div>
      {#if me.anyColor}
        <div class="custom">
          <input type="color" value={typed ?? me.color ?? '#145AFF'} oninput={(e) => (custom = e.currentTarget.value)} aria-label="Pick a color" disabled={busy} />
          <input class="field" bind:value={custom} maxlength="7" spellcheck="false" placeholder="#RRGGBB" aria-label="Any color" disabled={busy} />
          <Button small disabled={busy || !typed || typed === me.color?.toUpperCase()} onclick={() => typed && color(typed)}>Use</Button>
        </div>
      {:else}
        <p class="note">
          {namedColor(me.color) ?? (me.color ? 'Your color isn’t one of these.' : '')} Prime or Turbo can pick any color.
        </p>
      {/if}
    </section>

    {#if styles.length}
      <section>
        <h3 class="label">Smileys</h3>
        <div class="styles" role="radiogroup" aria-label="Smiley style">
          {#each styles as c (c.style)}
            {@const on = me.smilies.current === c.style}
            <button class="style" class:on role="radio" aria-checked={on} disabled={busy} onclick={() => !on && smile(c)}>
              <span class="faces">
                {#each c.preview as id (id)}<img src={img(id)} alt="" />{/each}
              </span>
              {STYLE_NAME[c.style]}
            </button>
          {/each}
        </div>
      </section>
    {/if}
  {/if}
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 16px;
    width: 300px;
    max-width: 100%;
    padding: 8px;
    overflow-y: auto;
  }
  .preview {
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 32px;
    padding: 6px 10px;
    background: var(--surface-hover);
    border-radius: var(--radius);
  }
  .preview img {
    width: 18px;
    height: 18px;
  }
  .preview b {
    font-weight: 700;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  h3 {
    margin: 0;
  }
  .sub {
    margin: 2px 0 0;
    color: var(--muted);
    font-size: 0.8rem;
  }
  .own {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-top: 4px;
  }
  .own span {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 0.85rem;
  }
  .own small {
    color: var(--muted);
    font-size: 0.8rem;
  }
  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  /* 15 colors: two even rows of 8 and 7. */
  .colors {
    display: grid;
    grid-template-columns: repeat(8, 28px);
    gap: 8px;
  }
  .swatch {
    width: 28px;
    height: 28px;
    padding: 0;
    border: 2px solid var(--border-strong);
    border-radius: 50%;
  }
  /* Picked: a ring inside the swatch, so it never looks like the focus ring around it. */
  .swatch.on {
    border-color: var(--text);
    box-shadow: inset 0 0 0 2px var(--elevated);
  }
  .swatch:focus-visible {
    outline-offset: 2px;
  }
  .custom {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .custom .field {
    width: 96px;
    font-family: var(--font-mono);
  }
  .custom input[type='color'] {
    width: 30px;
    height: 28px;
    padding: 0;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: none;
  }
  .badge {
    display: grid;
    place-items: center;
    min-width: 34px;
    height: 34px;
    padding: 3px;
    background: var(--surface-hover);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
  }
  .badge img {
    width: 24px;
    height: 24px;
  }
  .badge.none {
    padding: 0 10px;
    font-size: 0.8rem;
    color: var(--muted);
  }
  .badge.on,
  .style.on {
    border-color: var(--accent-border);
    background: var(--accent-dim);
    color: var(--text);
  }
  .styles {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 6px;
  }
  .style {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 6px 4px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.8rem;
  }
  .faces {
    display: flex;
    gap: 2px;
  }
  .faces img {
    width: 20px;
    height: 20px;
  }
  .note {
    margin: 0;
    color: var(--dim);
    font-size: 0.85rem;
  }
</style>
