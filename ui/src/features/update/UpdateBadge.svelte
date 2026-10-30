<!-- V105-20 A new version is out: a badge before Settings, and a pop-over with what's
     new, "Download and update", "Later" (asks again at the next start) and "Skip this version".
     V1061 The pop-over opens by itself when a version is found. Rust checks GitHub; the file
     must match its SHA-256 and carry a valid signature before it runs.
     P6 Laid out like a small dialog: title and the version you have, a footer with Skip on the
     left and Later, then the main action on the right. Opened by itself it leaves the focus
     where you are typing. -->
<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app';

  import Button from '../../components/Button.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Popover from '../../components/Popover.svelte';
  import { update } from './update.svelte';
  import { opensItself } from './updateState';

  let open = $state(false);
  /** Opened by itself: the focus stays put (a found version mustn't eat what you type). */
  let quiet = $state(false);
  let current = $state('');

  $effect(() => {
    void update.start();
    void getVersion()
      .then((v) => (current = v))
      .catch(() => {});
  });

  const u = $derived(update.available);

  // V1061 A version found opens the pop-over by itself, once per version this run; not after "Later".
  let shown: string | null = null;
  $effect(() => {
    const v = u?.version;
    if (!v || !opensItself(v, shown, update.later)) return;
    shown = v;
    quiet = !open;
    open = true;
  });

  function toggle() {
    quiet = false;
    open = !open;
  }

  function later() {
    open = false;
    update.deferLater();
  }

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
      onclick={toggle}
    />
    <Popover bind:open label="Update" flush focus={!quiet}>
      <div class="panel">
        <div class="body">
          <div class="title">
            <b>cyex deck {u.version}</b>
            {#if u.prerelease}<span class="chip">Pre-release</span>{/if}
          </div>
          <p class="sub">
            {#if current}You have {current}.{/if}
            <Button small variant="link" hit onclick={() => update.openNotes()}>What’s new</Button>
          </p>
          {#if update.busy}
            <div class="bar" role="progressbar" aria-label="Update" aria-valuenow={update.percent ?? 0} aria-valuemin={0} aria-valuemax={100}>
              <div class="fill" style:width="{update.percent ?? 0}%"></div>
            </div>
            <span class="status" aria-live="polite">{update.progress}</span>
          {/if}
          {#if update.error}<p class="error" role="alert">{update.error}</p>{/if}
        </div>
        {#if !update.busy}
          <div class="foot">
            <span class="skip"><Button variant="ghost" small onclick={skip}>Skip this version</Button></span>
            <Button small onclick={later}>Later</Button>
            <Button variant="primary" small onclick={main}>{u.canInstall ? 'Download and update' : 'Open release page'}</Button>
          </div>
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
    width: min(360px, calc(100vw - 16px));
    display: flex;
    flex-direction: column;
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px 12px 10px;
  }
  .title {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }
  /* A long pre-release tag wraps instead of pushing the chip out. */
  .title b {
    min-width: 0;
    font-size: 0.95rem;
    font-weight: 700;
    line-height: 1.3;
    overflow-wrap: anywhere;
  }
  .chip {
    flex: none;
    margin-top: 1px;
    padding: 2px 6px;
    border-radius: var(--radius);
    background: var(--neutral-chip);
    color: var(--muted);
    font-size: 0.62rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    line-height: 1.3;
    text-transform: uppercase;
  }
  .sub {
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 8px;
    color: var(--muted);
    font-size: var(--text-sub);
  }
  .bar {
    height: 4px;
    margin-top: 6px;
    border-radius: var(--radius-sm);
    background: var(--surface);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--accent-line);
    transition: width var(--base) ease-out;
  }
  .status {
    color: var(--muted);
    font-size: var(--text-hint);
    font-variant-numeric: tabular-nums;
  }
  .error {
    margin: 2px 0 0;
    color: var(--red);
    font-size: var(--text-hint);
    overflow-wrap: anywhere;
  }
  /* Wraps rather than spilling out when a long label meets a narrow window. */
  .foot {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    align-items: center;
    gap: 6px;
    padding: 8px 12px 10px;
    border-top: 1px solid var(--border);
  }
  /* Skip sits apart on the left, so it isn't hit for Later. */
  .skip {
    margin-right: auto;
    margin-left: -10px;
  }
</style>
