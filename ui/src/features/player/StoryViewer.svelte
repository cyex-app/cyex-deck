<!-- A channel's stories (1.1), one after the other, over the stream. Clicking the left or
     right side goes back or on. Closing marks what was shown as seen, like twitch.tv. -->
<script lang="ts">
  import { onDestroy } from 'svelte';

  import Avatar from '../../components/Avatar.svelte';
  import Icon from '../../components/Icon.svelte';
  import type { Story } from '../../ipc/bindings/Story';
  import { channelIpc } from '../../ipc/player';
  import { formatTime } from '../../lib/format';

  interface Props {
    channelId: string;
    account: string;
    name: string;
    avatar: string | null | undefined;
    onclose: () => void;
  }

  let { channelId, account, name, avatar, onclose }: Props = $props();

  /** Pictures show this long. */
  const PICTURE_MS = 6000;

  let stories = $state<Story[] | null>(null);
  let index = $state(0);
  let progress = $state(0);
  let problem = $state<string | null>(null);
  let seenUpTo = '';
  let timer: ReturnType<typeof setInterval> | undefined;

  // svelte-ignore state_referenced_locally
  channelIpc
    .stories(channelId, account)
    .then((list) => (stories = list))
    .catch((e) => (problem = String(e)));

  const story = $derived(stories?.[index]);

  $effect(() => {
    if (!story) return;
    if (story.createdAt > seenUpTo) seenUpTo = story.createdAt;
    progress = 0;
    if (story.video) return;
    const started = Date.now();
    timer = setInterval(() => {
      progress = (Date.now() - started) / PICTURE_MS;
      if (progress >= 1) next();
    }, 100);
    return () => clearInterval(timer);
  });

  onDestroy(() => {
    clearInterval(timer);
    if (seenUpTo) void channelIpc.storiesSeen(channelId, account, seenUpTo);
  });

  function next() {
    if (stories && index < stories.length - 1) index += 1;
    else onclose();
  }

  function prev() {
    if (index > 0) index -= 1;
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
    else if (e.key === 'ArrowRight') next();
    else if (e.key === 'ArrowLeft') prev();
  }
</script>

<svelte:window {onkeydown} />

<!-- The dimmed area closes the stories; arrows and Escape work too. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="overlay" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="phone" role="dialog" aria-label="Stories">
    {#if stories?.length && story}
      <div class="bars">
        {#each stories as s, i (s.id)}
          <span class="bar"><span style:width="{i < index ? 100 : i === index ? Math.min(progress, 1) * 100 : 0}%"></span></span>
        {/each}
      </div>
      <div class="who">
        <Avatar src={avatar} {name} size={24} />
        <b>{name}</b>
        <span>{formatTime(new Date(story.createdAt))}</span>
        {#if story.subsOnly}<span class="subs">Subs</span>{/if}
        <button class="close" aria-label="Close" onclick={onclose}><Icon name="x" size={16} /></button>
      </div>
      {#key story.id}
        {#if story.video}
          <!-- Stories have no captions track; the caption text is shown below. -->
          <!-- svelte-ignore a11y_media_has_caption -->
          <video
            src={story.video}
            poster={story.image ?? undefined}
            autoplay
            playsinline
            ontimeupdate={(e) => (progress = e.currentTarget.currentTime / (e.currentTarget.duration || 1))}
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
      <p class="empty">{problem ?? (stories ? 'No stories right now.' : 'Loading…')}</p>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: absolute;
    inset: 0;
    z-index: 6;
    display: grid;
    place-items: center;
    padding: 12px;
    background: rgba(0, 0, 0, 0.7);
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
    color: #fff;
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
    background: rgba(255, 255, 255, 0.3);
    border-radius: 2px;
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    background: #fff;
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
  .who :global(.avatar) {
    border-radius: 50%;
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
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    background: none;
    border: 0;
    color: #fff;
  }
  .caption {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 2;
    margin: 0;
    padding: 30px 14px 16px;
    background: linear-gradient(transparent, rgba(0, 0, 0, 0.7));
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
    color: #ccc;
  }
</style>
