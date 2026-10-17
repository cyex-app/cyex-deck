<!-- A channel's about page (1.21): bio, followers, last stream, links and panels.
     Extension panels can't be drawn here and are left out (V106-23). -->
<script lang="ts">
  import { SvelteSet } from 'svelte/reactivity';
  import Button from '../../components/Button.svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import Icon from '../../components/Icon.svelte';
  import type { About } from '../../ipc/bindings/About';
  import { channelIpc } from '../../ipc/player';
  import { linkLabel, linked, shows } from './panels';
  import { systemIpc } from '../../ipc/system';
  import { formatDate, formatNumber } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import StoryRing from '../player/StoryRing.svelte';
  import Markdown from './Markdown.svelte';
  import { errorText } from '../../lib/errors';
  import Loading from '../../components/Loading.svelte';
  import { tooltip } from '../../components/tooltip.svelte';

  interface Props {
    login: string;
    account: string | null;
    onstories: () => void;
    /** Streaming right now: the last stream is this one. */
    live?: boolean;
  }

  let { login, account, onstories, live = false }: Props = $props();

  let about = $state<About | null>(null);
  let problem = $state<string | null>(null);
  let retry = $state(false);
  let attempt = $state(0);
  const broken = new SvelteSet<string>();
  let bannerBroken = $state(false);

  $effect(() => {
    const l = login;
    void attempt;
    let alive = true;
    about = null;
    problem = null;
    retry = false;
    bannerBroken = false;
    channelIpc
      .about(l)
      .then((a) => alive && ((about = a), (problem = a ? null : 'This channel doesn’t exist.'), (retry = false)))
      .catch((e) => alive && ((problem = errorText(e, 'load this channel’s about page')), (retry = true)));
    return () => (alive = false);
  });

  function open(url: string) {
    systemIpc.openChatLink(url).catch((e) => toast.error(errorText(e)));
  }

  // V106-22 The whole panel is the link; links inside its text open themselves.
  function panelClick(e: MouseEvent, url: string) {
    if ((e.target as Element).closest('a')) return;
    open(url);
  }
  function panelKey(e: KeyboardEvent, url: string) {
    if (e.key === 'Enter' && e.target === e.currentTarget) open(url);
  }
</script>

<div class="page">
  {#if problem}
    <EmptyState icon="alert" text={problem}>
      {#if retry}<Button small onclick={() => attempt++}>Try again</Button>{/if}
    </EmptyState>
  {:else if !about}
    <Loading />
  {:else}
    <svelte:boundary onerror={(e) => console.error('about page', e)}>
    {#if about.banner && !bannerBroken}<img class="banner" src={about.banner} alt="" onerror={() => (bannerBroken = true)} />{/if}
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
          {#if about.partner}<span class="partner" use:tooltip={'Partner'}><Icon name="check" size={12} /></span>{/if}
        </h2>
        <span>{formatNumber(about.followers)} followers · joined {formatDate(new Date(about.createdAt))}</span>
      </div>
    </div>
    {#if about.bio}<p class="bio">{about.bio}</p>{/if}
    {#if about.lastStream}
      <p class="last">
        {live ? 'Live now' : `Last live ${formatDate(new Date(about.lastStream.startedAt))}`}{about.lastStream.title ? `: ${about.lastStream.title}` : ''}
        {#if about.lastStream.game}<span>({about.lastStream.game})</span>{/if}
      </p>
    {/if}
    {#if about.socials.length}
      <div class="socials">
        {#each about.socials as s, i (i)}
          <button onclick={() => open(s.url)}><Icon name="link" size={12} />{s.title}</button>
        {/each}
      </div>
    {/if}
    {#if about.panels.length}
      <div class="panels">
        {#each about.panels.filter((p) => shows(p, broken)) as p, i (i)}
          {#if linked(p)}
            <!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role: the whole panel is the link (V106-22) -->
            <section
              class="panel linked"
              role="link"
              tabindex="0"
              aria-label={linkLabel(p)}
              onclick={(e) => panelClick(e, p.link!)}
              onkeydown={(e) => panelKey(e, p.link!)}
            >
              {#if p.title}<h3>{p.title}</h3>{/if}
              {#if p.image && !broken.has(p.image)}
                <img src={p.image} alt="" loading="lazy" onerror={() => broken.add(p.image!)} />
              {/if}
              {#if p.body.length}<Markdown blocks={p.body} />{/if}
            </section>
          {:else}
            <section class="panel">
              {#if p.title}<h3>{p.title}</h3>{/if}
              {#if p.image && !broken.has(p.image)}
                <img src={p.image} alt={p.title ?? ''} loading="lazy" onerror={() => broken.add(p.image!)} />
              {/if}
              {#if p.body.length}<Markdown blocks={p.body} />{/if}
            </section>
          {/if}
        {/each}
      </div>
    {/if}
    {#snippet failed(_e, reset)}
      <EmptyState icon="alert" text="Couldn’t show this page.">
        <Button small onclick={() => { reset(); attempt++; }}>Try again</Button>
      </EmptyState>
    {/snippet}
    </svelte:boundary>
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
    height: auto;
    margin-bottom: 8px;
    border-radius: var(--radius);
  }
  .panel {
    border-radius: var(--radius);
  }
  .panel.linked,
  .panel.linked :global(*) {
    cursor: pointer;
  }
  .panel.linked:hover h3 {
    text-decoration: underline;
  }
  .panel.linked:hover img {
    filter: brightness(1.08);
  }
  .panel.linked:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 4px;
  }
</style>
