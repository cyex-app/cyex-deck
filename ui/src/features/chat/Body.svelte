<!-- Message fragments (1.8): text, emotes, mentions, links (X2), cheers, GIFs (1.43).
     Clicks and tooltips are handled by the list through data attributes. -->
<script lang="ts">
  import type { Fragment } from '../../ipc/bindings/Fragment';
  import { formatNumber } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import { catalogs } from './catalog.svelte';
  import { cheerImg, cheerTier, emoteImg } from './urls';

  let { fragments, channel }: { fragments: Fragment[]; channel: string } = $props();

  const theme = $derived(settings.v.appearance.theme);
  const gifs = $derived(settings.v.chat.gifs);
</script>

{#each fragments as f, i (i)}
  {#if f.t === 'text'}{f.text}{:else if f.t === 'emote'}
    {@const img = emoteImg(f.provider, f.id, theme)}
    <img class="emote" src={img.src} srcset={img.srcset} alt={f.name} data-tip={f.name} loading="lazy" decoding="async" />
  {:else if f.t === 'mention'}<span class="mention" role="link" tabindex="-1" data-user={f.login}>{f.text}</span>
  {:else if f.t === 'link'}<a class="link" href={f.url} data-link={f.url} data-host={f.host}>{f.text}</a>
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
    {#if gifs === 'show'}<img class="gif" src={f.url} alt="GIF" data-tip="GIF" loading="lazy" decoding="async" />
    {:else if gifs === 'link'}<a class="link" href={f.url} data-link={f.url} data-host="GIF">GIF</a>
    {:else}[GIF]{/if}
  {/if}
{/each}

<style>
  .emote {
    height: calc(var(--chat-size) * 2.15);
    margin: -0.45em 0 -0.35em;
    vertical-align: middle;
  }
  .gif {
    display: block;
    max-width: min(240px, 100%);
    max-height: 160px;
    margin: 4px 0;
    border-radius: var(--radius);
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
