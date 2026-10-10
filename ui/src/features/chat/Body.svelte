<!-- Message fragments (1.8): text, emotes, mentions, links (X2), cheers, GIFs (1.43).
     A gigantified message (1.42) draws its last Twitch emote big. Clicks and tooltips
     are handled by the list through data attributes. -->
<script lang="ts">
  import type { Fragment } from '../../ipc/bindings/Fragment';
  import { formatNumber } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import { catalogs } from './catalog.svelte';
  import ChatEmote from './ChatEmote.svelte';
  import { saved } from './saved';
  import { cheerImg, cheerTier } from './urls';

  let { fragments, channel, giant = false }: { fragments: Fragment[]; channel: string; giant?: boolean } = $props();

  const theme = $derived(settings.v.appearance.theme);
  const gifs = $derived(settings.v.chat.gifs);
  /** 1.42 Only when the message ends in a Twitch emote, like twitch.tv. */
  const big = $derived.by(() => {
    if (!giant || !settings.v.chat.effects) return -1;
    let last = fragments.length - 1;
    while (last >= 0 && fragments[last].t === 'text' && !(fragments[last] as { text: string }).text.trim()) last--;
    const f = fragments[last];
    return f?.t === 'emote' && f.provider === 'twitch' ? last : -1;
  });
  /** 1.43 GIPHY's page for a GIF. */
  const giphyPage = (id: string) => `https://giphy.com/gifs/${encodeURIComponent(id)}`;
  /** The smaller webp GIPHY keeps next to the gif. */
  const webp = (url: string) => url.replace(/\/giphy\.gif(\?|$)/, '/giphy.webp$1');
</script>

{#each fragments as f, i (i)}
  {#if f.t === 'text'}{f.text}{:else if f.t === 'emote'}{#if i === big}<span class="giant"><ChatEmote {f} /></span>{:else}<ChatEmote
        {f}
      />{/if}{:else if f.t === 'mention'}<span class="mention" role="link" tabindex="-1" data-user={f.login}>{f.text}</span>
  {:else if f.t === 'link'}<a
      class="link"
      href={f.url}
      data-link={f.url}
      data-host={f.host}
      data-target={f.target ? JSON.stringify(f.target) : undefined}>{f.text}</a
    >
  {:else if f.t === 'cheer'}
    {@const c = catalogs.cheermote(channel, f.prefix)}
    {@const tier = c && cheerTier(c, f.amount)}
    <span class="cheer" style:color={tier?.color} data-tip="{formatNumber(f.amount)} bits">
      {#if c && tier}
        {@const img = cheerImg(c, tier, theme)}
        <img class="emote" src={img.src} srcset={img.srcset} alt={f.prefix} loading="lazy" decoding="async" />{f.amount}
      {:else}{f.prefix}{f.amount}{/if}
    </span>
  {:else if f.t === 'gif'}
    {#if gifs === 'show'}<a class="gif-wrap" href={giphyPage(f.id)} data-link={giphyPage(f.id)} data-host="giphy.com"
        ><img class="gif" src={saved(`gif:${f.id}`) ?? webp(f.url)} alt="GIF" loading="lazy" decoding="async" referrerpolicy="no-referrer" /></a
      >
    {:else if gifs === 'link'}<a class="link" href={giphyPage(f.id)} data-link={giphyPage(f.id)} data-host="giphy.com">GIF</a>
    {:else}<i>[GIF]</i>{/if}
  {/if}
{/each}

<style>
  .gif-wrap {
    display: block;
    width: fit-content;
    margin: 4px 0;
  }
  .gif {
    display: block;
    max-width: min(320px, 100%);
    max-height: 140px;
    border-radius: 4px;
  }
  .gif-wrap:hover .gif {
    filter: brightness(0.85);
  }
  /* 1.42 Twitch draws a gigantified emote at 112 px. */
  .giant {
    display: block;
  }
  .giant :global(.emote) {
    height: 112px;
    margin: 4px 0;
  }
  .mention {
    font-weight: 700;
    cursor: pointer;
  }
  .mention:hover {
    text-decoration: underline;
  }
  .link {
    color: var(--accent-text);
    text-decoration: none;
    word-break: break-all;
  }
  .link:hover {
    text-decoration: underline;
  }
  .cheer {
    font-weight: 700;
    white-space: nowrap;
  }
</style>
