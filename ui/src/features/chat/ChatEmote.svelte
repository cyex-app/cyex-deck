<!-- One emote in a chat line, with zero-width emotes stacked on top and modifiers (C7).
     Shown as its name when its kind is turned off (2.3), it's unlisted and hidden (2.4) or
     the user hid it (E4). Clicks and the bigger hover picture (E6) go through the list.
     One picture URL per emote and moving modifiers on one clock keep copies in step (V105-08). -->
<script lang="ts">
  import type { Fragment } from '../../ipc/bindings/Fragment';
  import type { Layer } from '../../ipc/bindings/Layer';
  import type { Provider } from '../../ipc/bindings/Provider';
  import { focus } from '../../lib/focus.svelte';
  import { density } from '../../lib/density.svelte';
  import { settings } from '../../lib/settings.svelte';
  import { cyextvOn } from '../../lib/cyextv';
  import { PROVIDER_NAMES, emoteRatio, modLook, type ModLook } from '../emotes/emotes';
  import { hidden } from '../emotes/hidden.svelte';
  import { inStep, moves } from '../emotes/phase';
  import { emoteImg, emoteLarge } from './urls';
  import { isSmiley, smileys } from '../emotes/smiley';
  import { height1x, sizeShare, smallSeen, smallShare } from '../emotes/drawSize';

  type EmoteFragment = Extract<Fragment, { t: 'emote' }>;

  /** `giant`: 1.42 the emote this message gigantified, for the card. `bare`: V111-01 an emote-change line's
   *  picture, its name already in the line's text: nothing when hidden, no alt text. */
  let { f, giant = false, bare = false }: { f: EmoteFragment; giant?: boolean; bare?: boolean } = $props();

  const c = $derived(settings.v.chat);
  const theme = $derived(settings.v.appearance.theme);
  const still = $derived(focus.still);

  function shows(provider: Provider, id: string): boolean {
    // V110-E13 Each service has its own switch.
    const on = { twitch: c.twitchEmotes, seventv: c.seventvEmotes, bttv: c.bttvEmotes, ffz: c.ffzEmotes, cyextv: cyextvOn(c.cyextvEmotes) }[provider];
    if (!on) return false;
    return !hidden.has(provider, id);
  }

  /** V110-N18 A Twitch smiley draws a bit smaller; known after its first picture loads. */
  function smiley(img: HTMLImageElement, id: string) {
    if (!isSmiley(img.currentSrc || img.src, img.naturalHeight)) return;
    smileys.add(id);
    img.classList.add('smiley');
  }

  /** V111-44 A 7TV, BTTV or FFZ picture smaller than a full emote draws at its own size; the
   *  zero-width emotes on it follow, so they stay over it. */
  function small(img: HTMLImageElement, e: Layer) {
    const h = height1x(e.provider, img.currentSrc || img.src, img.naturalHeight);
    const share = h == null ? null : smallShare(e.provider, h);
    if (share == null) return;
    smallSeen.set(`${e.provider}:${e.id}`, share);
    const now = Math.min(sizeShare(e.provider, e.size), share);
    for (const el of img.closest('.emote-wrap')?.querySelectorAll<HTMLElement>('img.emote') ?? [img]) {
      el.style.setProperty('--eh', String(now));
      el.classList.toggle('short', now < 1);
    }
  }

  /** V111-44 Share of a full emote's height: shorter when wider than 3:1 or small (drawSize.ts). */
  function share(e: Layer): number {
    return Math.min(sizeShare(e.provider, e.size), smallSeen.get(`${e.provider}:${e.id}`) ?? 1);
  }

  const visible = $derived(shows(f.provider, f.id) && !(f.unlisted && c.unlistedEmotes === 'hide'));
  /** V111-20 BTTV and FFZ effects, unless turned off: then BTTV's words show as typed. */
  const look = $derived(modLook(c.emoteEffects ? f.mods : undefined, f.size));
  const pre = $derived(c.emoteEffects && visible ? '' : (f.pre ?? ''));
  const layers = $derived((f.stack ?? []).filter((l) => shows(l.provider, l.id)));
  const card = $derived(
    JSON.stringify({
      emote: { provider: f.provider, id: f.id, name: f.name, animated: f.animated, size: f.size },
      stack: f.stack ?? [],
      ...(giant && { giant: true }),
    }),
  );
</script>

<!-- V111-20 `fx`: the base emote's modifiers. `copy`: FFZ Slide's second picture, right behind the first.
     V111-44 `on`: the base under a zero-width emote, whose height it takes. -->
{#snippet picture(e: Layer, classes: string, fx: ModLook | null = null, copy = false, on: Layer | null = null)}
  {@const img = emoteImg(e.provider, e.id, theme, { animated: e.animated, still, scale: density.scale })}
  {@const eh = share(on ?? e)}
  {@const anim = fx?.animation ? (copy ? fx.animation.replace('emote-slide ', 'emote-slide-2 ') : fx.animation) : ''}
  <img
    class="emote {classes}"
    class:smiley={e.provider === 'twitch' && smileys.has(e.id)}
    class:fx-move={anim !== ''}
    class:short={eh < 1}
    src={img.src}
    alt={bare || copy ? '' : e.name}
    aria-hidden={copy ? 'true' : undefined}
    onload={(ev) => (e.provider === 'twitch' ? smiley(ev.currentTarget as HTMLImageElement, e.id) : !on && small(ev.currentTarget as HTMLImageElement, e))}
    style:aspect-ratio={fx?.ratio ? `${fx.ratio} / 1` : emoteRatio(e.size)}
    style:--fx-anim={anim || undefined}
    style:--eh={eh < 1 ? String(eh) : undefined}
    loading="lazy"
    decoding="async"
    use:inStep={moves(classes)}
  />
{/snippet}

{#if !visible}{#if !bare}{pre}{f.name}{/if}{#if !bare}{#each f.stack ?? [] as l, i (i)}{' '}{l.name}{/each}{/if}{:else}
  {@const unlisted = f.unlisted && c.unlistedEmotes === 'blur'}
  {@const base = `${look.classes}${unlisted ? ' unlisted' : ''}`}
  {pre}<span
    class:stack={layers.length > 0}
    class="emote-wrap"
    role="button"
    tabindex="-1"
    data-emote={card}
    data-unlisted={f.unlisted ? '' : undefined}
    data-tip={[c.emoteEffects && f.pre?.trim(), f.name, ...layers.map((l) => l.name)].filter(Boolean).join(' ')}
    data-tip-img={emoteLarge(f.provider, f.id, theme, { animated: f.animated, still })}
    data-hint={PROVIDER_NAMES[f.provider]}
    >{#if look.slide}<span class="slide-box">{@render picture(f, base, look)}{@render picture(f, base, look, true)}</span
      >{:else}{@render picture(f, base, look)}{/if}{#each layers as l, i (i)}{@render picture(l, '', null, false, f)}{/each}</span
  >
{/if}

<style>
  .emote-wrap {
    cursor: pointer;
  }
</style>
