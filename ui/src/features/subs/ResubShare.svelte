<!-- 1.28 Sharing a resub in chat, like twitch.tv's "Share your resub": an optional
     message and the streak if there is one. -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Checkbox from '../../components/Checkbox.svelte';
  import type { ResubShare } from '../../ipc/bindings/ResubShare';
  import { subsIpc } from '../../ipc/subs';
  import { toast } from '../../lib/toast.svelte';
  import { months } from './labels';
  import { errorText } from '../../lib/errors';

  interface Props {
    account: string;
    login: string;
    resub: ResubShare;
    onshared: () => void;
  }

  let { account, login, resub, onshared }: Props = $props();

  let message = $state('');
  let streak = $state(true);
  let busy = $state(false);

  async function share() {
    busy = true;
    try {
      await subsIpc.shareResub(account, login, resub.id, message, streak && resub.streak !== null);
      toast.info('Shared in chat.');
      onshared();
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = false;
    }
  }
</script>

<div class="share">
  <p>
    {resub.gift ? 'Your gifted sub' : 'Your sub'} is at {months(resub.months)}{#if resub.streak}, {months(resub.streak)} in a
      row{/if}. Share it in chat?
  </p>
  <textarea bind:value={message} rows="2" maxlength="500" placeholder="Add a message (optional)"></textarea>
  <div class="row">
    {#if resub.streak}<Checkbox checked={streak} label="Show my streak" onchange={(v) => (streak = v)} />{/if}
    <span class="grow"></span>
    <Button small variant="primary" icon="star" disabled={busy} onclick={share}>Share</Button>
  </div>
</div>

<style>
  .share {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  p {
    margin: 0;
    color: var(--text);
  }
  textarea {
    resize: vertical;
    min-height: 44px;
    padding: 6px 8px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--text);
    font: inherit;
  }
  textarea:focus {
    outline: none;
    border-color: var(--accent-border);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .grow {
    flex: 1;
  }
</style>
