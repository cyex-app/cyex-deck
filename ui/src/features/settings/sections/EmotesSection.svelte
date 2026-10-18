<!-- Emotes, badges and paints (2.3, 2.4, C11, E2, E3, E4, D21). Stored in the chat
     partition. -->
<script lang="ts">
  import SectionHead from '../SectionHead.svelte';
  import Icon from '../../../components/Icon.svelte';
  import Row from '../../../components/Row.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { EmoteSource } from '../../../ipc/bindings/EmoteSource';
  import type { UnlistedEmotes } from '../../../ipc/bindings/UnlistedEmotes';
  import { settings } from '../../../lib/settings.svelte';
  import { PROVIDER_NAMES } from '../../emotes/emotes';
  import { hidden } from '../../emotes/hidden.svelte';

  const c = $derived(settings.v.chat);

  const BADGES = [
    { key: 'twitch', label: 'Twitch' },
    { key: 'seventv', label: '7TV' },
    { key: 'bttv', label: 'BTTV' },
    { key: 'ffz', label: 'FFZ' },
  ] as const;
  const UNLISTED: { value: UnlistedEmotes; label: string }[] = [
    { value: 'hide', label: 'Hide' },
    { value: 'blur', label: 'Blur' },
    { value: 'show', label: 'Show' },
  ];
  const SIZES = [
    { value: '75', label: 'Small' },
    { value: '100', label: 'Normal' },
    { value: '150', label: 'Large' },
    { value: '200', label: 'Huge' },
  ] as const;

  const name = (s: EmoteSource) => (s === 'emoji' ? 'Emoji' : PROVIDER_NAMES[s]);

  function move(i: number, by: number) {
    const order = [...c.emotePriority];
    const j = i + by;
    if (j < 0 || j >= order.length) return;
    [order[i], order[j]] = [order[j], order[i]];
    void settings.patch('chat', { emotePriority: order });
  }

  const size = $derived(
    (SIZES.find((s) => Number(s.value) === c.emoteSize)?.value ?? '100') as (typeof SIZES)[number]['value'],
  );
</script>

<SectionHead title="Emotes and badges" level={2} />

<SectionHead title="Emotes">Where emotes come from and how big they are.</SectionHead>
<Row setting="chat.twitchEmotes" label="Twitch emotes">
  <Toggle label="Twitch emotes" checked={c.twitchEmotes} onchange={(v) => settings.patch('chat', { twitchEmotes: v })} />
</Row>
<Row setting="chat.thirdPartyEmotes" label="7TV, BTTV and FFZ emotes" hint="Extra emote sets made by the community.">
  <Toggle
    label="7TV, BTTV and FFZ emotes"
    checked={c.thirdPartyEmotes}
    onchange={(v) => settings.patch('chat', { thirdPartyEmotes: v })}
  />
</Row>
<Row setting="chat.unlistedEmotes" label="Unlisted 7TV emotes" hint="Emotes 7TV hasn’t approved for public listing.">
  <Segmented
    label="Unlisted 7TV emotes"
    options={UNLISTED}
    value={c.unlistedEmotes}
    onchange={(v) => settings.patch('chat', { unlistedEmotes: v })}
  />
</Row>
<Row setting="chat.emoteSize" label="Size">
  <Segmented label="Emote size" options={SIZES} value={size} onchange={(v) => settings.patch('chat', { emoteSize: Number(v) })} />
</Row>
<Row setting="chat.animateOnlyFocused" label="Only move while the window is focused" hint="Emotes, badges and paints stand still otherwise. Saves some power.">
  <Toggle
    label="Only move while the window is focused"
    checked={c.animateOnlyFocused}
    onchange={(v) => settings.patch('chat', { animateOnlyFocused: v })}
  />
</Row>
<Row setting="chat.emotePriority" label="Which wins" hint="When two have an emote with the same name. The channel’s emotes still beat global ones.">
  <ol class="order">
    {#each c.emotePriority as s, i (s)}
      <li>
        <span>{i + 1}. {name(s)}</span>
        <button aria-label="Move {name(s)} up" disabled={i === 0} onclick={() => move(i, -1)}>
          <Icon name="chevronUp" size={14} />
        </button>
        <button aria-label="Move {name(s)} down" disabled={i === c.emotePriority.length - 1} onclick={() => move(i, 1)}>
          <Icon name="chevronDown" size={14} />
        </button>
      </li>
    {/each}
  </ol>
</Row>
<Row setting="chat.hiddenEmotes" label="Hidden emotes" hint="Right-click an emote in chat to hide it. Hidden ones show as text." stacked>
  {#if c.hiddenEmotes.length}
    <div class="chips">
      {#each c.hiddenEmotes as e (`${e.source}:${e.id}`)}
        <span class="chip">
          {e.name}
          <span class="src">{name(e.source)}</span>
          <button aria-label="Show {e.name} again" onclick={() => hidden.show(e.source, e.id)}>×</button>
        </span>
      {/each}
    </div>
  {:else}
    <span class="none">None.</span>
  {/if}
</Row>

<SectionHead title="Badges">The small icons before a name.</SectionHead>
{#each BADGES as b (b.key)}
  <Row setting="chat.badges.{b.key}" label="{b.label} badges">
    <Toggle
      label="{b.label} badges"
      checked={c.badges[b.key]}
      onchange={(v) => settings.patch('chat', { badges: { [b.key]: v } })}
    />
  </Row>
{/each}
<Row setting="chat.paints.seventv" label="7TV name paints" hint="Colorful names for 7TV users.">
  <Toggle
    label="7TV name paints"
    checked={c.paints.seventv}
    onchange={(v) => settings.patch('chat', { paints: { seventv: v } })}
  />
</Row>

<SectionHead title="Sharing">Let other cyex users see that you’re here.</SectionHead>
<Row
  setting="chat.sharePresence"
  label="Send presence to 7TV and BTTV"
  hint="Others see your 7TV paint, badge and personal emotes. Only the chats you’re in are sent."
>
  <Toggle
    label="Send presence to 7TV and BTTV"
    checked={c.sharePresence}
    onchange={(v) => settings.patch('chat', { sharePresence: v })}
  />
</Row>

<style>
  .order {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .order li {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .order span {
    width: 64px;
    font-size: 0.85rem;
  }
  .order button {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--muted);
  }
  .order button:disabled {
    opacity: 0.4;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 4px 0 10px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    font-size: 0.82rem;
  }
  .src {
    color: var(--dim);
    font-size: var(--text-hint);
  }
  .chip button {
    width: 20px;
    height: 20px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--dim);
  }
  .chip button:hover {
    color: var(--red);
  }
  .none {
    color: var(--dim);
    font-size: var(--text-sub);
  }
</style>
