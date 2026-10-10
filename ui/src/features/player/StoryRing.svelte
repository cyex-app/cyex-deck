<!-- A channel's avatar with a ring when it has stories (1.1): colored while some aren't
     seen. Stories need the tile's GQL sign-in; without it this is just the avatar. -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { StoryRing } from '../../ipc/bindings/StoryRing';
  import { channelIpc } from '../../ipc/player';
  import { avatars } from '../emotes/avatars.svelte';

  interface Props {
    channelId: string;
    account: string | null;
    avatar: string | null | undefined;
    name: string;
    /** For the channel's 7TV picture (C23). */
    login?: string;
    size?: number;
    /** Opens the stories; without stories the avatar does `onplain`. */
    onopen: () => void;
    onplain?: () => void;
  }

  let { channelId, account, avatar: twitch, name, login, size = 24, onopen, onplain }: Props = $props();

  const avatar = $derived(avatars.of(login) ?? twitch);

  let ring = $state<StoryRing | null>(null);

  $effect(() => {
    if (!account) return;
    let alive = true;
    channelIpc
      .storyRing(channelId, account)
      .then((r) => alive && (ring = r))
      .catch(() => {});
    return () => (alive = false);
  });
</script>

{#if ring}
  <button class="ring" class:unseen={ring.unseen} style:--size="{size}px" use:tooltip={{ label: 'Stories', hint: 'Short posts from the streamer.' }} onclick={onopen}>
    <Avatar src={avatar} {name} size={size - 6} />
  </button>
{:else if onplain}
  <button class="plain" style:--size="{size}px" use:tooltip={'About'} onclick={onplain}>
    <Avatar src={avatar} {name} {size} />
  </button>
{:else}
  <Avatar src={avatar} {name} {size} />
{/if}

<style>
  button {
    flex: none;
    display: grid;
    place-items: center;
    width: var(--size);
    height: var(--size);
    padding: 0;
    background: none;
    border: 0;
    border-radius: 50%;
  }
  .ring {
    box-shadow: inset 0 0 0 2px var(--dim);
  }
  .ring.unseen {
    box-shadow: inset 0 0 0 2px var(--twitch);
  }
  .ring :global(.avatar),
  .plain :global(.avatar) {
    border-radius: 50%;
  }
</style>
