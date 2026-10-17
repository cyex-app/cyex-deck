<!-- A channel's stories (1.1), one after the other, over the stream. Clicking the left or
     right side goes back or on. Closing marks what was shown as seen, like twitch.tv. -->
<script lang="ts">
  import { onDestroy } from 'svelte';

  import Avatar from '../../components/Avatar.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import type { Story } from '../../ipc/bindings/Story';
  import { channelIpc } from '../../ipc/player';
  import { layer } from '../../lib/escape';
  import { focusScope } from '../../lib/focusScope';
  import { formatTime } from '../../lib/format';
  import { applySink } from './output.svelte';
  import { sounding } from '../tabs/sounding.svelte';
  import { errorText } from '../../lib/errors';
  import Loading from '../../components/Loading.svelte';
  import { pictureBar, videoBar } from './storyBar';

  interface Props {
    channelId: string;
    account: string;
    name: string;
    avatar: string | null | undefined;
    onclose: () => void;
    /** V106-25 Its tab is muted. */
    muted?: boolean;
    /** V106-25 The tile it plays over, for the tab's sound badge. */
    tileId?: string;
  }

  let { channelId, account, name, avatar, onclose, muted = false, tileId }: Props = $props();

  let storyVideo = $state<HTMLVideoElement>();
  // V1051-01 Stories play on the streams' device too.
  $effect(() => {
    if (storyVideo) applySink(storyVideo);
  });
  // V106-25 A playing story shows on its tab's sound badge.
  $effect(() => {
    const v = storyVideo;
    if (!v || !tileId) return;
    return sounding.watch(tileId, v, () => true);
  });

  /** Pictures show this long. */
  const PICTURE_MS = 6000;

  let stories = $state<Story[] | null>(null);
  let index = $state(0);
  let problem = $state<string | null>(null);
  let seenUpTo = '';
  /** V106-20 The current story's bar fill. */
  let fill = $state<HTMLElement>();

  // svelte-ignore state_referenced_locally
  channelIpc
    .stories(channelId, account)
    .then((list) => (stories = list))
    .catch((e) => (problem = errorText(e)));

  const story = $derived(stories?.[index]);

  // V106-20 One compositor animation per story; pictures end it, videos drive it.
  $effect(() => {
    if (!story || !fill) return;
    if (story.createdAt > seenUpTo) seenUpTo = story.createdAt;
    if (!story.video) return pictureBar(fill, PICTURE_MS, next);
    if (!storyVideo) return;
    return videoBar(fill, storyVideo);
  });

  onDestroy(() => {
    if (seenUpTo) void channelIpc.storiesSeen(channelId, account, seenUpTo);
  });

  function next() {
    if (stories && index < stories.length - 1) index += 1;
    else onclose();
  }

  function prev() {
    if (index > 0) index -= 1;
  }

  let phone = $state<HTMLElement>();

  // The shared layer: Escape closes only the top one, Tab stays inside, focus goes back.
  $effect(() => {
    if (!phone) return;
    const release = focusScope(phone, { trap: true });
    const off = layer('dialog', onclose);
    return () => {
      off();
      release();
    };
  });

  function onkeydown(e: KeyboardEvent) {
    if (e.defaultPrevented) return;
    if (e.key === 'ArrowRight') next();
    else if (e.key === 'ArrowLeft') prev();
  }
</script>

<svelte:window {onkeydown} />

<!-- The dimmed area closes the stories; arrows and Escape work too. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="overlay" onmousedown={(e) => e.target === e.currentTarget && onclose()}>
  <div class="phone" bind:this={phone} role="dialog" aria-modal="true" aria-label="Stories" tabindex="-1">
    {#if stories?.length && story}
      <div class="bars">
        {#each stories as s, i (s.id)}
          <span class="bar">
            {#if i === index}<span class="fill now" bind:this={fill}></span>
            {:else}<span class="fill" class:done={i < index}></span>{/if}
          </span>
        {/each}
      </div>
      <div class="who">
        <Avatar src={avatar} {name} size={24} />
        <b>{name}</b>
        <span>{formatTime(new Date(story.createdAt))}</span>
        {#if story.subsOnly}<span class="subs">Subs</span>{/if}
        <div class="close"><IconButton icon="x" label="Close" size={16} onclick={onclose} /></div>
      </div>
      {#key story.id}
        {#if story.video}
          <!-- Stories have no captions track; the caption text is shown below. -->
          <!-- svelte-ignore a11y_media_has_caption -->
          <video
            bind:this={storyVideo}
            src={story.video}
            {muted}
            poster={story.image ?? undefined}
            autoplay
            playsinline
            onended={next}
          ></video>
        {:else}
          <img src={story.image} alt="" />
        {/if}
      {/key}
      {#if story.caption}<p class="caption">{story.caption}</p>{/if}
      <button class="half left" aria-label="Back" onclick={prev}></button>
      <button class="half right" aria-label="Next" onclick={next}></button>
    {:else}
      {#if !problem && !stories}<Loading video />{:else}<p class="empty">{problem ?? 'No stories right now.'}</p>{/if}
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: absolute;
    inset: 0;
    z-index: var(--z-panel);
    display: grid;
    place-items: center;
    padding: 12px;
    background: var(--scrim-strong);
  }
  .phone {
    position: relative;
    height: 100%;
    max-height: 100%;
    aspect-ratio: 9 / 16;
    max-width: 100%;
    display: flex;
    background: #000;
    border-radius: var(--radius);
    overflow: hidden;
    color: var(--on-video);
  }
  video,
  img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .bars {
    position: absolute;
    top: 6px;
    left: 8px;
    right: 8px;
    z-index: 2;
    display: flex;
    gap: 3px;
  }
  .bar {
    flex: 1;
    height: 3px;
    background: var(--on-video-faint);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }
  /* V106-20 transform only, so the bar never triggers layout. */
  .fill {
    display: block;
    height: 100%;
    background: var(--on-video);
    transform: scaleX(0);
    transform-origin: left center;
  }
  .fill.done {
    transform: none;
  }
  .fill.now {
    will-change: transform;
  }
  .who {
    position: absolute;
    top: 14px;
    left: 8px;
    right: 4px;
    z-index: 3;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.85rem;
  }
  .who span {
    opacity: 0.8;
  }
  .subs {
    padding: 0 5px;
    background: var(--twitch);
    border-radius: var(--radius);
    font-size: 0.7rem;
    font-weight: 700;
    opacity: 1 !important;
  }
  .close {
    margin-left: auto;
  }
  .close :global(.icon-btn) {
    color: var(--on-video);
  }
  .close :global(.icon-btn:hover:not(:disabled)) {
    background: var(--scrim);
    color: var(--on-video);
  }
  .phone:focus {
    outline: none;
  }
  .caption {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 2;
    margin: 0;
    padding: 30px 14px 16px;
    background: linear-gradient(transparent, var(--scrim-strong));
    text-align: center;
  }
  .half {
    position: absolute;
    top: 50px;
    bottom: 0;
    z-index: 1;
    width: 50%;
    background: none;
    border: 0;
  }
  .left {
    left: 0;
  }
  .right {
    right: 0;
  }
  .empty {
    margin: auto;
    padding: 20px;
    text-align: center;
    color: var(--on-video-dim);
  }
</style>
