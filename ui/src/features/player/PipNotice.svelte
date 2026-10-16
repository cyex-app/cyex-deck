<!-- Over a video that plays in its own small window: the browser's plain "playing in picture
     in picture" text is replaced by the channel and a way back. -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';

  let { name, avatar }: { name: string; avatar?: string | null } = $props();
</script>

<div class="pip" role="status">
  <Avatar src={avatar} {name} size={56} />
  <span>{name} plays in a small window.</span>
  <Button small onclick={() => void document.exitPictureInPicture().catch(() => {})}>Bring it back</Button>
</div>

<style>
  .pip {
    position: absolute;
    inset: 0;
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 16px;
    background: #000;
    color: var(--on-video-dim);
    text-align: center;
  }
</style>
