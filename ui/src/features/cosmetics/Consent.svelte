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
  <section class="card">
    <h2>Use 7TV with {name}?</h2>
    {@render text()}
    <div class="row">{@render buttons()}</div>
  </section>
{:else}
  <Dialog title="Use 7TV with {name}?" width={420} onclose={oncancel}>
    {@render text()}
    {#snippet footer()}{@render buttons()}{/snippet}
  </Dialog>
{/if}

<style>
  .card {
    max-width: 420px;
    margin: 48px auto;
    padding: 16px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
  }
  p {
    margin: 0 0 10px;
    color: var(--muted);
    line-height: 1.5;
  }
  .row {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 14px;
  }
</style>
