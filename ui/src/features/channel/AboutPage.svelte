<!-- A channel's about page (1.21): bio, followers, last stream, links and panels.
     Extension panels are web pages Twitch runs; they open on twitch.tv. -->
<script lang="ts">
  import EmptyState from '../../components/EmptyState.svelte';
  import Icon from '../../components/Icon.svelte';
  import type { About } from '../../ipc/bindings/About';
  import { channelIpc } from '../../ipc/player';
  import { systemIpc } from '../../ipc/system';
  import { formatDate, formatNumber } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import StoryRing from '../player/StoryRing.svelte';
  import Markdown from './Markdown.svelte';
  import { errorText } from '../../lib/errors';
  import Loading from '../../components/Loading.svelte';

  interface Props {
    login: string;
    account: string | null;
    onstories: () => void;
  }

  let { login, account, onstories }: Props = $props();

  let about = $state<About | null>(null);
  let problem = $state<string | null>(null);

  $effect(() => {
    const l = login;
    let alive = true;
    about = null;
    problem = null;
    channelIpc
      .about(l)
      .then((a) => alive && ((about = a), (problem = a ? null : 'This channel doesn’t exist.')))
      .catch((e) => alive && (problem = errorText(e)));
    return () => (alive = false);
  });

  function open(url: string, twitch = false) {
    (twitch ? systemIpc.openLink(url) : systemIpc.openChatLink(url)).catch((e) => toast.error(errorText(e)));
  }
</script>

<div class="page">
  {#if problem}
    <EmptyState icon="alert" text={problem} />
  {:else if !about}
    <Loading />
  {:else}
    {#if about.banner}<img class="banner" src={about.banner} alt="" />{/if}
    <div class="head">
      <StoryRing
        channelId={about.id}
        {account}
        avatar={about.avatar}
        name={about.name}
        login={about.login}
        size={64}
        onopen={onstories}
      />
      <div class="who">
        <h2>
          {about.name}
          {#if about.partner}<span class="partner" title="Partner"><Icon name="check" size={12} /></span>{/if}
        </h2>
        <span>{formatNumber(about.followers)} followers · joined {formatDate(new Date(about.createdAt))}</span>
      </div>
    </div>
    {#if about.bio}<p class="bio">{about.bio}</p>{/if}
    {#if about.lastStream}
      <p class="last">
        Last live {formatDate(new Date(about.lastStream.startedAt))}{about.lastStream.title ? `: ${about.lastStream.title}` : ''}
        {#if about.lastStream.game}<span>({about.lastStream.game})</span>{/if}
      </p>
    {/if}
    {#if about.socials.length}
      <div class="socials">
        {#each about.socials as s (s.url)}
          <button onclick={() => open(s.url)}><Icon name="link" size={12} />{s.title}</button>
        {/each}
      </div>
    {/if}
    {#if about.panels.length}
      <div class="panels">
        {#each about.panels as p, i (i)}
          <section class="panel">
            {#if p.extension !== null}
              <p class="ext">
                Extension{p.extension ? `: ${p.extension}` : ''}.
                <button class="text-link" onclick={() => open(`https://www.twitch.tv/${about!.login}/about`, true)}>Open on Twitch</button>
              </p>
            {:else}
              {#if p.title}<h3>{p.title}</h3>{/if}
              {#if p.image}
                {#if p.link}
                  <button class="img-link" onclick={() => open(p.link!)}><img src={p.image} alt={p.title ?? ''} loading="lazy" /></button>
                {:else}
                  <img src={p.image} alt={p.title ?? ''} loading="lazy" />
                {/if}
              {/if}
              {#if p.body.length}<Markdown blocks={p.body} />{/if}
            {/if}
          </section>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .page {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 0 16px;
  }
  .banner {
    display: block;
    width: 100%;
    aspect-ratio: 5 / 2;
    max-height: 180px;
    object-fit: cover;
    background: var(--surface-hover);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 16px 6px;
  }
  .who {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    font-size: 1.15rem;
  }
  .who span {
    color: var(--dim);
    font-size: 0.8rem;
  }
  .partner {
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
    background: var(--twitch);
    border-radius: 50%;
    color: #fff;
  }
  .bio,
  .last {
    margin: 6px 16px;
    white-space: pre-line;
  }
  .last {
    color: var(--muted);
    font-size: 0.85rem;
  }
  .last span {
    color: var(--dim);
  }
  .socials {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 10px 16px;
  }
  .socials button {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 3px 9px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    font-size: 0.8rem;
  }
  .socials button:hover {
    border-color: var(--border-strong);
  }
  .panels {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 12px;
    margin: 14px 16px 0;
    align-items: start;
  }
  .panel h3 {
    margin: 0 0 6px;
    font-size: 0.95rem;
  }
  .panel img {
    display: block;
    max-width: 100%;
    min-height: 64px;
    margin-bottom: 8px;
    background: var(--surface-hover);
    border-radius: var(--radius);
  }
  .img-link {
    padding: 0;
    background: none;
    border: 0;
  }
  .ext {
    margin: 0;
    color: var(--dim);
    font-size: 0.85rem;
  }
  .text-link {
    padding: 0;
    background: none;
    border: 0;
    color: var(--accent-text);
  }
</style>
