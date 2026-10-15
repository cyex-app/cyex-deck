<!-- V104-01 / V105-17 A chosen sound file in a mention or highlight row: its name (cut with …), play, and back to the chime. -->
<script lang="ts">
  import IconButton from '../../../components/IconButton.svelte';
  import { mentionSound } from '../../../lib/sound';
  import { fileName } from './status';

  interface Props {
    path: string;
    onclear: () => void;
  }

  let { path, onclear }: Props = $props();
</script>

<span class="file" title={path}>
  <span class="name">{fileName(path)}</span>
  <IconButton icon="play" label="Play" size={12} onclick={() => void mentionSound(path)} />
  <IconButton icon="x" label="Use the chime" size={12} onclick={onclear} />
</span>

<style>
  .file {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    max-width: 180px;
    min-width: 0;
    color: var(--muted);
    white-space: nowrap;
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .file :global(.icon-btn) {
    flex: none;
    width: 22px;
    height: 22px;
  }
  /* 24 px to hit, 22 px to see. */
  .file :global(.icon-btn)::before {
    content: "";
    position: absolute;
    inset: -1px;
  }
</style>
