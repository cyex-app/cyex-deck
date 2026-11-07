<!-- About: the version, a check for updates, and who made it. -->
<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app';

  import Button from '../../../components/Button.svelte';
  import Row from '../../../components/Row.svelte';
  import { updateIpc } from '../../../ipc/update';
  import { systemIpc } from '../../../ipc/system';
  import { errorText } from '../../../lib/errors';
  import { toast } from '../../../lib/toast.svelte';
  import { checkText } from '../../update/updateText';
  import { STORE } from '../../accounts/cyexBadges';
  import SectionHead from '../SectionHead.svelte';

  let version = $state('');
  $effect(() => {
    void getVersion()
      .then((v) => (version = v))
      .catch(() => {});
  });

  // Builds before 1.0 are pre-releases.
  const pre = $derived(version.startsWith('0.'));

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
    }
  }

  function open(e: MouseEvent, url: string) {
    e.preventDefault();
    systemIpc.openLink(url).catch((err) => toast.error(errorText(err)));
  }
</script>

<SectionHead title="About" level={2} />

<Row
  setting="about:checkNow"
  label={version ? `cyex deck ${version}` : 'cyex deck'}
  hint={pre ? 'A pre-release. Look for a newer one right away.' : 'Look for a newer one right away.'}
>
  {#if checked}<span class="checked">{checked}</span>{/if}
  <Button small disabled={checking} onclick={() => void checkNow()}>{checking ? 'Checking…' : 'Check now'}</Button>
</Row>
<p class="note">Checks GitHub for new versions. Sends nothing about you or your accounts.</p>

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
  .checked {
    color: var(--muted);
    font-size: var(--text-sub);
  }
  .note {
    margin: 8px 0 0;
    color: var(--dim);
    font-size: var(--text-hint);
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
