<!-- Emotes, badges and paints (2.3, 2.4, C11, E2, E3, E4, D21, V110-E13). Stored in the chat
     partition, except the extra badge sets (V108-N06), which are in emotes. One group per
     provider holds its switches. -->
<script lang="ts">
  import SectionHead from '../SectionHead.svelte';
  import Icon from '../../../components/Icon.svelte';
  import Row from '../../../components/Row.svelte';
  import Segmented from '../../../components/Segmented.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { EmoteSource } from '../../../ipc/bindings/EmoteSource';
  import type { UnlistedEmotes } from '../../../ipc/bindings/UnlistedEmotes';
  import { settings } from '../../../lib/settings.svelte';
  import { GROUPS } from '../emoteGroups';
  import ProviderGroup from './ProviderGroup.svelte';
  import { PROVIDER_NAMES } from '../../emotes/emotes';
  import { hidden } from '../../emotes/hidden.svelte';

  const c = $derived(settings.v.chat);

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

  /** V110-E13 Providers switched off aren't ranked; the rest keep their order. V110-U71 Twitch is ranked too. */
  const ON: Record<string, boolean> = $derived({
    seventv: c.seventvEmotes,
    bttv: c.bttvEmotes,
    ffz: c.ffzEmotes,
  });
  const ranked = $derived(c.emotePriority.filter((s) => ON[s] !== false));

  function move(i: number, by: number) {
    const j = i + by;
    if (j < 0 || j >= ranked.length) return;
    const order = [...c.emotePriority];
    const a = order.indexOf(ranked[i]);
    const b = order.indexOf(ranked[j]);
    [order[a], order[b]] = [order[b], order[a]];
    void settings.patch('chat', { emotePriority: order });
  }

  const size = $derived(
    (SIZES.find((s) => Number(s.value) === c.emoteSize)?.value ?? '100') as (typeof SIZES)[number]['value'],
  );
</script>

<SectionHead title="Emotes and badges" level={2} page="emotes" />

<SectionHead title="Emotes">How big they are and which wins. Which services they come from is under Providers.</SectionHead>
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
<Row setting="chat.emotePriority" label="Which wins" hint="First wins when two have an emote with the same name, in chat and in suggestions. The channel’s emotes still beat global ones.">
  <ol class="order">
    {#each ranked as s, i (s)}
      <li>
        <span>{i + 1}. {name(s)}</span>
        <button aria-label="Move {name(s)} up" disabled={i === 0} onclick={() => move(i, -1)}>
          <Icon name="chevronUp" size={14} />
        </button>
        <button aria-label="Move {name(s)} down" disabled={i === ranked.length - 1} onclick={() => move(i, 1)}>
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

<SectionHead title="Providers">Emotes, badges and name paints from each service. The switch on a line is for all of them.</SectionHead>
{#each GROUPS as g (g.key)}
  <ProviderGroup group={g}>
    {#snippet extra()}
      {#if g.key === 'seventv'}
        <Row setting="chat.paints.outlineLight" label="Outline paints in light themes" hint="A thin dark edge, so pale paints stay readable.">
          <Toggle
            label="Outline paints in light themes"
            checked={c.paints.outlineLight}
            disabled={!c.paints.seventv}
            onchange={(v) => settings.patch('chat', { paints: { outlineLight: v } })}
          />
        </Row>
      {/if}
    {/snippet}
  </ProviderGroup>
{/each}

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
    position: relative;
    width: 20px;
    height: 20px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--dim);
  }
  /* V110-F31 24 px to hit. */
  .chip button::before {
    content: '';
    position: absolute;
    inset: -3px;
  }
  .chip button:hover {
    color: var(--red);
  }
  .none {
    color: var(--dim);
    font-size: var(--text-sub);
  }
</style>
