<!-- V105-20 A new version is out: a quiet badge before Settings, and a pop-over with what's
     new, "Download and update", "Later" and "Skip this version". Rust checks GitHub; the file
     is checked against its SHA-256 before it runs. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Popover from '../../components/Popover.svelte';
  import { update } from './update.svelte';

  let open = $state(false);

  $effect(() => {
    void update.start();
  });

  const u = $derived(update.available);

  function main() {
    if (u?.canInstall) void update.install();
    else update.openNotes();
  }

  function skip() {
    open = false;
    void update.skip();
  }
</script>

{#if u}
  <div class="wrap">
    <IconButton
      icon="download"
      label="Update available"
      hint={`cyex deck ${u.version} is out.`}
      badge
      active={open}
      onclick={() => (open = !open)}
    />
    <Popover bind:open>
      <div class="panel">
        <div class="title">
          <b>cyex deck {u.version}</b>
          {#if u.prerelease}<span class="chip">Pre-release</span>{/if}
        </div>
        <button class="link" onclick={() => update.openNotes()}>What's new</button>
        {#if update.busy}
          <div class="bar"><div class="fill" style:width="{update.percent ?? 0}%"></div></div>
          <span class="status">{update.progress}</span>
        {/if}
        {#if update.error}<span class="error">{update.error}</span>{/if}
        {#if !update.busy}
          <div class="buttons">
            <Button variant="primary" small onclick={main}>{u.canInstall ? 'Download and update' : 'Open release page'}</Button>
            <Button variant="ghost" small onclick={() => (open = false)}>Later</Button>
          </div>
          <div class="skip"><Button variant="ghost" small onclick={skip}>Skip this version</Button></div>
        {/if}
      </div>
    </Popover>
  </div>
{/if}

<style>
  .wrap {
    position: relative;
  }
  .panel {
    width: 260px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .title b {
    font-weight: 600;
  }
  .chip {
    padding: 1px 6px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--dim);
    font-size: 0.72rem;
  }
  .link {
    align-self: flex-start;
    padding: 0;
    background: none;
    border: 0;
    color: var(--accent-text);
    font-size: 0.85rem;
  }
  .link:hover {
    text-decoration: underline;
  }
  .bar {
    height: 4px;
    border-radius: 2px;
    background: var(--surface);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--accent);
  }
  .status {
    color: var(--muted);
    font-size: 0.82rem;
  }
  .error {
    color: var(--red);
    font-size: 0.82rem;
  }
  .buttons {
    display: flex;
    gap: 8px;
  }
  .skip {
    display: flex;
    justify-content: flex-end;
  }
</style>
