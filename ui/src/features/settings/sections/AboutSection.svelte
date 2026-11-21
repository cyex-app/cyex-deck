<!-- About: the version, updates (V112-22), and who made it. -->
<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app';

  import Button from '../../../components/Button.svelte';
  import Row from '../../../components/Row.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import { updateIpc } from '../../../ipc/update';
  import { systemIpc } from '../../../ipc/system';
  import { errorText } from '../../../lib/errors';
  import { formatDate, formatTime } from '../../../lib/format';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import { update } from '../../update/update.svelte';
  import { checkText, displayVersion, releaseNotesUrl } from '../../update/updateText';
  import { STORE } from '../../accounts/cyexBadges';
  import SectionHead from '../SectionHead.svelte';
  import ConnectionHealth from './ConnectionHealth.svelte';

  let version = $state('');
  $effect(() => {
    void getVersion()
      .then((v) => (version = v))
      .catch(() => {});
    void update.start().then(() => update.refresh());
  });

  const up = $derived(settings.v.general.updates);
  const found = $derived(update.available);

  // Builds before 1.0 are pre-releases.
  const pre = $derived(version.startsWith('0.'));

  // V112-22 yyyy/mm/dd HH:mm, from the last answer GitHub gave, timed or by hand.
  const lastChecked = $derived.by(() => {
    if (update.checked === null) return 'Not yet this session';
    const d = new Date(update.checked * 1000);
    return `${formatDate(d)} ${formatTime(d)}`;
  });

  // V105-20 "Check now": the answer sits next to the button.
  let checking = $state(false);
  let checked = $state('');
  async function checkNow() {
    checking = true;
    checked = '';
    try {
      checked = checkText(await updateIpc.check(), version);
    } catch {
      checked = checkText({ kind: 'failed' });
    } finally {
      checking = false;
      void update.refresh();
    }
  }

  // V112-22 "Show again" clears the skip and looks once more, so the badge comes back at once.
  async function showAgain() {
    await settings.patch('general', { updates: { ...up, skip: '' } });
    void checkNow();
  }

  function open(e: MouseEvent, url: string) {
    e.preventDefault();
    systemIpc.openLink(url).catch((err) => toast.error(errorText(err)));
  }
</script>

<SectionHead title="About" level={2} page="about" />

<Row label={version ? `cyex deck ${version}` : 'cyex deck'} hint={pre ? 'A pre-release.' : 'The version you are running.'}>
  {#if pre}<span class="chip">Pre-release</span>{/if}
</Row>

<SectionHead title="Updates" />
<Row setting="about:checkNow" label="Check now" hint="Asks GitHub for a newer one right away. Sends nothing about you or your accounts.">
  {#if checked}<span class="checked">{checked}</span>{/if}
  <Button small disabled={checking} onclick={() => void checkNow()}>{checking ? 'Checking…' : 'Check now'}</Button>
</Row>
<Row setting="about:lastChecked" label="Last checked" hint="Timed checks count too.">
  <span class="when">{lastChecked}</span>
</Row>
{#if found}
  <!-- V112-22 The badge's own install, here too. -->
  <Row
    setting="about:install"
    label="{displayVersion(found.version)} is out"
    hint={update.error || update.progress || (found.canInstall ? 'Downloads, checks the file and restarts.' : 'Opens the release page.')}
  >
    {#if update.downloading}
      <Button small onclick={() => update.stop()}>Stop</Button>
    {:else}
      <Button
        small
        variant="primary"
        disabled={update.busy}
        onclick={() => (found.canInstall ? void update.install() : update.openNotes())}
      >
        {found.canInstall ? 'Download and update' : 'Open release page'}
      </Button>
    {/if}
  </Row>
{/if}
<Row setting="general.updates.check" label="Check for updates" hint="At startup and every 8 hours.">
  <Toggle
    label="Check for updates"
    checked={up.check}
    onchange={(v) => settings.patch('general', { updates: { ...up, check: v } })}
  />
</Row>
<Row setting="general.updates.prereleases" label="Include pre-releases" hint="Test builds that come before a release.">
  <Toggle
    label="Include pre-releases"
    checked={up.prereleases}
    onchange={(v) => settings.patch('general', { updates: { ...up, prereleases: v } })}
  />
</Row>
{#if up.skip}
  <Row setting="general.updates.skip" label="Skipped: {displayVersion(up.skip)}" hint="Left out of the timed checks.">
    <Button small onclick={() => void showAgain()}>Show again</Button>
  </Row>
{/if}
<Row setting="about:notes" label="What’s new" hint="This version’s release notes on GitHub.">
  <Button small disabled={!version} onclick={(e) => open(e, releaseNotesUrl(version))}>Release notes</Button>
</Row>

<ConnectionHealth />

<!-- V109-B02 -->
<Row setting="about:badge" label="cyex badges" hint="Founder, supporter or supporter+. Pick yours under cyex cosmetics in the account menu.">
  <Button small onclick={(e) => open(e, STORE)}>Get a badge</Button>
</Row>

<Row setting="about:credit" label="Made by" hint="The rest of cyex lives on the homepage.">
  <span class="credit">
    <a href="https://seryxx.com" onclick={(e) => open(e, 'https://seryxx.com')}>seryxx</a>
    <span class="sep" aria-hidden="true">·</span>
    <a href="https://cyex.app" onclick={(e) => open(e, 'https://cyex.app')}>cyex.app</a>
  </span>
</Row>

<style>
  .chip {
    padding: 2px 6px;
    border-radius: var(--radius);
    background: var(--neutral-chip);
    color: var(--muted);
    font-size: 0.62rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .when {
    color: var(--muted);
    font: var(--text-sub) var(--font-mono);
    font-variant-numeric: tabular-nums;
  }
  .checked {
    color: var(--muted);
    font-size: var(--text-sub);
  }
  .credit {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: var(--text-sub);
  }
  a {
    color: var(--muted);
    text-decoration: underline;
    text-decoration-color: var(--border-strong);
    text-underline-offset: 2px;
  }
  a:hover {
    color: var(--text);
    text-decoration-color: currentColor;
  }
  .sep {
    color: var(--dim);
  }
</style>
