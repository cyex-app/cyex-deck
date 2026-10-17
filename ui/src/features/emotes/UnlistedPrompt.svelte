<!-- V106-30 The emote card's one-time choice for unlisted 7TV emotes. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import { keepBlurred, showUnlisted } from './unlisted';

  let { onchosen }: { onchosen: () => void } = $props();
</script>

<section class="unlisted-prompt"aria-label="Unlisted emotes">
  <p>Unlisted emotes are blurred in every chat.</p>
  <Button
    variant="primary"
    small
    onclick={() => {
      showUnlisted();
      onchosen();
    }}>Show unlisted emotes</Button
  >
  <button
    class="keep"
    type="button"
    onclick={() => {
      keepBlurred();
      onchosen();
    }}>Keep them blurred</button
  >
</section>

<style>
  .unlisted-prompt {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 12px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }
  p {
    margin: 0;
    color: var(--dim);
    font-size: 0.78rem;
  }
  .unlisted-prompt :global(.btn) {
    width: 100%;
    justify-content: center;
  }
  .keep {
    align-self: center;
    padding: 2px 4px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.78rem;
    cursor: pointer;
  }
  .keep:hover {
    color: var(--text);
  }
  .keep:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 1px;
  }
</style>
