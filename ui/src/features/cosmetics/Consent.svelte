<!-- V1051-02 The yes before anything talks to 7TV. A dialog in the pop-up, in place of the
     panel in the view. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Dialog from '../../components/Dialog.svelte';

  interface Props {
    name: string;
    inline: boolean;
    onallow: () => void;
    oncancel: () => void;
  }

  let { name, inline, onallow, oncancel }: Props = $props();

  // V1051-02 The inline card takes focus (on Allow) and Escape means Cancel, like the dialog.
  let row = $state<HTMLElement>();
  $effect(() => {
    row?.querySelector<HTMLElement>('button:last-of-type')?.focus();
  });
</script>

{#snippet text()}
  <p>cyex deck will sign in to 7TV as {name}, read the paints and badges you own, and change the ones you wear when you pick one.</p>
  <p>Your 7TV sign-in stays in Windows Credential Manager on this PC. It’s never in exports or backups.</p>
  <p>You can take this back any time in Settings → Accounts.</p>
{/snippet}

{#snippet buttons()}
  <Button variant="ghost" onclick={oncancel}>Cancel</Button>
  <Button variant="primary" onclick={onallow}>Allow</Button>
{/snippet}

{#if inline}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <section
    class="card"
    aria-label="Use 7TV with {name}?"
    onkeydown={(e) => {
      if (e.key !== 'Escape') return;
      e.stopPropagation();
      oncancel();
    }}
  >
    <h2>Use 7TV with {name}?</h2>
    <div class="copy">{@render text()}</div>
    <div class="row" bind:this={row}>{@render buttons()}</div>
  </section>
{:else}
  <Dialog title="Use 7TV with {name}?" width={420} onsubmit={onallow} onclose={oncancel}>
    {@render text()}
    {#snippet footer()}{@render buttons()}{/snippet}
  </Dialog>
{/if}

<style>
  .card {
    display: flex;
    flex-direction: column;
    max-width: 420px;
    /* V1 Shrinks with the panel (a flex column), so the copy scrolls and the buttons always show. */
    min-height: 0;
    margin: clamp(16px, 8vh, 64px) auto 16px;
    padding: 16px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
  }
  h2 {
    flex: none;
  }
  .copy {
    min-height: 0;
    overflow-y: auto;
  }
  p {
    margin: 0 0 10px;
    color: var(--muted);
    line-height: 1.5;
    text-wrap: pretty;
  }
  .row {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    flex: none;
    margin-top: 14px;
  }
</style>
