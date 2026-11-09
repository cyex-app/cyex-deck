<!-- V108-C25 A link's card inside its tip (1.52), after FFZ's rich cards: site line, avatar,
     title and subtitle, the picture at its own shape (blurred when the page says NSFW), facts.
     A skeleton while it loads, a short line when there's nothing. -->
<script lang="ts">
  import { factText, kindText, numeric } from '../features/chat/linkFacts';
  import { previewPic } from '../features/chat/previewPic';
  import type { TipCard } from './tooltip.svelte';

  let { c }: { c: TipCard } = $props();
</script>

{#if c.state === 'loading'}
  <div class="card tip-card" role="status" aria-label="Loading preview">
    <span class="bar" style:width="30%"></span>
    <span class="bar tall" style:width="85%"></span>
    <span class="bar" style:width="60%"></span>
  </div>
{:else if c.state === 'none'}
  <p class="none tip-card">No preview</p>
{:else}
  {@const p = c.card}
  <!-- V110-F75 Pictures load only from the resolver's proxy. -->
  {@const pic = previewPic(p.thumbnail)}
  {@const face = previewPic(p.avatar)}
  {@const facts = p.facts.map((f) => ({ text: factText(f), num: numeric(f) })).filter((f) => f.text)}
  <!-- V110-C66 With a picture the card is as wide as the picture size asked for. -->
  <div
    class="card tip-card"
    class:live={p.live}
    style:min-width={pic && c.thumbWidth ? `${Math.max(200, c.thumbWidth)}px` : undefined}
  >
    <!-- V110-F46 A web page's site is its host, which the tip's "Opens x.com" already says. -->
    {#if p.kind !== 'web'}
      <span class="site">
        {kindText(p)}
        {#if p.kind === 'channel' || (p.kind === 'video' && p.live)}
          <span class="state" class:on={p.live}>{p.live ? 'Live' : 'Offline'}</span>
        {/if}
      </span>
    {/if}
    {#if p.title || p.subtitle || face}
      <div class="head">
        {#if face}<img class="avatar" src={face} alt="" referrerpolicy="no-referrer" />{/if}
        <div class="names">
          {#if p.title}<b class="title">{p.title}</b>{/if}
          {#if p.subtitle}<span class="sub">{p.subtitle}</span>{/if}
        </div>
      </div>
    {/if}
    {#if pic && c.thumbWidth}
      <div class="pic" class:blur={c.blur} style:max-width="{c.thumbWidth}px">
        <img src={pic} alt="" referrerpolicy="no-referrer" />
        {#if c.blur}<span class="flag">NSFW</span>{/if}
      </div>
    {/if}
    {#if facts.length}
      <p class="facts">
        {#each facts as f, i (i)}{#if i}<span class="dot" aria-hidden="true"> · </span>{/if}<span class:num={f.num}
            >{f.text}</span
          >{/each}
      </p>
    {/if}
    {#each p.lines as line, i (i)}
      {#if line.length}
        <p class="line">{#each line as part, j (j)}{#if part.bold}<b>{part.text}</b>{:else}{part.text}{/if}{/each}</p>
      {:else}<hr />{/if}
    {/each}
  </div>
{/if}

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 200px;
    margin-top: 4px;
    padding: 2px 0 2px 9px;
    box-shadow: inset 2px 0 0 var(--border-strong);
    font-size: 0.78rem;
    color: var(--muted);
  }
  .card.live {
    box-shadow: inset 2px 0 0 var(--live);
  }
  .bar {
    display: block;
    height: 0.7em;
    background: var(--surface-hover);
    border-radius: var(--radius-sm);
  }
  .bar.tall {
    height: 0.9em;
  }
  .none {
    margin: 2px 0 0;
    font-size: 0.76rem;
    color: var(--dim);
  }
  .site {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-label);
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--dim);
    overflow-wrap: anywhere;
  }
  .state {
    letter-spacing: 0.06em;
  }
  .state.on {
    color: var(--live);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .avatar {
    flex: none;
    width: 28px;
    height: 28px;
    border-radius: 50%;
  }
  .names {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .title,
  .sub {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
    overflow-wrap: anywhere;
  }
  .title {
    font-size: 0.8rem;
    font-weight: 700;
    color: var(--text);
  }
  .pic {
    position: relative;
    overflow: hidden;
    border-radius: var(--radius);
  }
  .pic img {
    display: block;
    width: 100%;
    max-height: 200px;
    object-fit: contain;
  }
  .blur img {
    filter: blur(14px);
  }
  .flag {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-size: var(--text-label);
    font-weight: 700;
    letter-spacing: 0.06em;
    color: var(--text);
  }
  .facts,
  .line {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .facts {
    color: var(--dim);
  }
  .num {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    font-size: 0.95em;
  }
  .line b {
    color: var(--text);
  }
  hr {
    margin: 2px 0;
    border: 0;
    border-top: 1px solid var(--border);
  }
</style>
