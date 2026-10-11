<!-- One emote in a chat line, with zero-width emotes stacked on top and modifiers (C7).
     Shown as its name when its kind is turned off (2.3), it's unlisted and hidden (2.4) or
     the user hid it (E4). Clicks and the bigger hover picture (E6) go through the list. -->
<script lang="ts">
  import type { Fragment } from '../../ipc/bindings/Fragment';
  import type { Layer } from '../../ipc/bindings/Layer';
  import type { Provider } from '../../ipc/bindings/Provider';
  import { focus } from '../../lib/focus.svelte';
  import { settings } from '../../lib/settings.svelte';
  import { PROVIDER_NAMES, emoteRatio, modClasses } from '../emotes/emotes';
  import { hidden } from '../emotes/hidden.svelte';
  import { emoteImg, emoteLarge } from './urls';

  type EmoteFragment = Extract<Fragment, { t: 'emote' }>;

  let { f }: { f: EmoteFragment } = $props();

  const c = $derived(settings.v.chat);
  const theme = $derived(settings.v.appearance.theme);
  const still = $derived(focus.still);

  function shows(provider: Provider, id: string): boolean {
    if (provider === 'twitch' ? !c.twitchEmotes : !c.thirdPartyEmotes) return false;
    return !hidden.has(provider, id);
  }

  const visible = $derived(shows(f.provider, f.id) && !(f.unlisted && c.unlistedEmotes === 'hide'));
  const layers = $derived((f.stack ?? []).filter((l) => shows(l.provider, l.id)));
  const card = $derived(
    JSON.stringify({
      emote: { provider: f.provider, id: f.id, name: f.name, animated: f.animated, size: f.size },
      stack: f.stack ?? [],
    }),
  );
</script>

{#snippet picture(e: Layer, classes: string)}
  {@const img = emoteImg(e.provider, e.id, theme, { animated: e.animated, still })}
  <img
    class="emote {classes}"
    src={img.src}
    srcset={img.srcset}
    alt={e.name}
    style:aspect-ratio={emoteRatio(e.size)}
    loading="lazy"
    decoding="async"
  />
{/snippet}

{#if !visible}{f.name}{#each f.stack ?? [] as l, i (i)}{' '}{l.name}{/each}{:else}
  {@const unlisted = f.unlisted && c.unlistedEmotes === 'blur'}
  {@const base = `${modClasses(f.mods)}${unlisted ? ' unlisted' : ''}`}
  <span
    class:stack={layers.length > 0}
    class="emote-wrap"
    role="button"
    tabindex="-1"
    data-emote={card}
    data-unlisted={f.unlisted ? '' : undefined}
    data-tip={[f.name, ...layers.map((l) => l.name)].join(' ')}
    data-tip-img={emoteLarge(f.provider, f.id, theme, { animated: f.animated, still })}
    data-hint={PROVIDER_NAMES[f.provider]}
    >{@render picture(f, base)}{#each layers as l, i (i)}{@render picture(l, '')}{/each}</span
  >
{/if}

<style>
  .emote-wrap {
    cursor: pointer;
  }
</style>
