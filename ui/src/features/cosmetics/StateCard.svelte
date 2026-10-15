<!-- V1051-02 What the menu says when it has nothing to show: no 7TV account, or 7TV down. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import { systemIpc } from '../../ipc/system';

  interface Props {
    kind: 'noAccount' | 'down';
    onretry: () => void;
  }

  let { kind, onretry }: Props = $props();
</script>

<section class="card">
  {#if kind === 'noAccount'}
    <p>7TV adds emotes, paints and badges to Twitch chat. Sign in once on 7tv.app, then come back here to pick your paint and badge.</p>
    <Button onclick={() => void systemIpc.openLink('https://7tv.app')}>Open 7tv.app</Button>
  {:else}
    <p>7TV isn’t answering. Try again in a minute.</p>
    <Button onclick={onretry}>Try again</Button>
  {/if}
</section>

<style>
  .card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
    max-width: 420px;
    margin: 48px auto;
    padding: 16px;
    background: var(--elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  p {
    margin: 0;
    color: var(--muted);
    line-height: 1.5;
  }
</style>
