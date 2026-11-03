<!-- V108-C36 One unban request in full, as Twitch's mod view shows it: who asks (picture,
     account age, followers, sub, follow age, from the usercard's Facts), what they wrote and
     when, their ban (reason, who, when; who can be hidden from the header's eye), their counts
     from the mod logs, recent messages (logs.cyex.app, else Twitch's mod logs: the usercard's
     Timeline), mod comments, their earlier requests, then a note and Approve or Deny. -->
<script lang="ts">
  import { untrack } from 'svelte';

  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';
  import Loading from '../../components/Loading.svelte';
  import type { ModHistory } from '../../ipc/bindings/ModHistory';
  import type { PastUnban } from '../../ipc/bindings/PastUnban';
  import type { Profile } from '../../ipc/bindings/Profile';
  import type { UnbanRequest } from '../../ipc/bindings/UnbanRequest';
  import { chatIpc } from '../../ipc/chat';
  import { modIpc } from '../../ipc/moderation';
  import { errorText } from '../../lib/errors';
  import { formatDate, formatNumber, formatTime } from '../../lib/format';
  import Comments from '../usercard/Comments.svelte';
  import Facts from '../usercard/Facts.svelte';
  import Timeline from '../usercard/Timeline.svelte';
  import { bannedBy, messageCount, statusText } from './unban';

  interface Props {
    account: string;
    /** The channel. */
    login: string;
    request: UnbanRequest;
    /** Who banned them shows as "Hidden". */
    hideBy: boolean;
    busy: boolean;
    onresolve: (approve: boolean, note: string) => void;
  }

  let { account, login, request, hideBy, busy, onresolve }: Props = $props();

  // One request for the component's life (the list keys it).
  // svelte-ignore state_referenced_locally
  const r = request;

  let profile = $state<Profile | null | undefined>(undefined);
  let history = $state<ModHistory | null | undefined>(undefined);
  let historyError = $state('');
  let past = $state.raw<PastUnban[] | null>(null);
  let note = $state('');

  chatIpc
    .profile(r.login)
    .then((p) => (profile = p))
    .catch(() => (profile = null));

  function readHistory() {
    modIpc
      .history(account, login, r.userId)
      .then((h) => {
        history = h;
        historyError = '';
      })
      .catch((e) => {
        if (history === undefined) history = null;
        historyError = errorText(e);
      });
  }
  untrack(readHistory);

  untrack(() =>
    modIpc
      .unbanPast(account, login, r.userId)
      .then((p) => (past = p.filter((x) => x.id !== r.id)))
      .catch(() => (past = [])),
  );

  /** Date and 24 h time; empty when unreadable. */
  const when = (iso: string | null | undefined) => {
    if (!iso) return '';
    const d = new Date(iso);
    return Number.isNaN(d.getTime()) ? '' : `${formatDate(d)} ${formatTime(d)}`;
  };

  const ban = $derived(history?.ban ?? null);
  const banWhen = $derived.by(() => {
    if (!ban) return '';
    const since = when(ban.since);
    const until = when(ban.until);
    if (until) return since ? `${since}, until ${until}` : `Until ${until}`;
    return since;
  });
</script>

<div class="detail">
  <div class="scroll">
    <header>
      <Avatar src={profile?.avatar} name={r.display || r.login} size={40} />
      <div class="who">
        <b class="name"><bdi>{r.display || r.login}</bdi></b>
        {#if r.display && r.display.toLowerCase() !== r.login}<span class="login">{r.login}</span>{/if}
      </div>
    </header>

    <section>
      <h4>Request <span class="t">{when(r.createdAt)}</span></h4>
      {#if r.text}<p class="msg" dir="auto">{r.text}</p>{:else}<p class="dim">No message.</p>{/if}
    </section>

    <section>
      <h4>Ban</h4>
      {#if history === undefined}
        <Loading rows={2} />
      {:else if history === null}
        <p class="dim">{historyError || 'Couldn’t load their mod history.'} <Button small variant="link" onclick={readHistory}>Retry</Button></p>
      {:else if ban}
        <dl>
          <dt>Reason</dt>
          <dd dir="auto">{ban.reason || 'No reason given'}</dd>
          <dt>By</dt>
          <dd class:dim={hideBy}><bdi>{bannedBy(ban.by, hideBy)}</bdi></dd>
          {#if banWhen}
            <dt>When</dt>
            <dd class="t">{banWhen}</dd>
          {/if}
        </dl>
      {:else}
        <p class="dim">Not banned right now.</p>
      {/if}
    </section>

    {#if profile}
      <section class="facts">
        <h4>Account</h4>
        {#key profile.id}<Facts {profile} channel={login} />{/key}
      </section>
    {/if}

    {#if history}
      <section>
        <h4>Here</h4>
        <div class="counts">
          <span><b>{messageCount(history.messageCount)}</b> messages</span>
          <span><b>{formatNumber(history.timeouts)}</b> timeouts</span>
          <span><b>{formatNumber(history.bans)}</b> bans</span>
          <span><b>{formatNumber(history.warnings)}</b> warnings</span>
        </div>
      </section>
    {/if}

    <section>
      <h4>Recent messages</h4>
      <div class="box msgs">
        <Timeline channel={login} userId={r.userId} login={r.login} name={r.display || r.login} live={[]} modAccount={account} {account} />
      </div>
    </section>

    {#if history}
      <section>
        <h4>Mod comments</h4>
        <div class="box comments">
          <Comments comments={history.comments} channel={login} {account} targetId={r.userId} onchange={readHistory} />
        </div>
      </section>
    {/if}

    {#if past?.length}
      <section>
        <h4>Earlier requests</h4>
        <ul class="past">
          {#each past as p (p.id)}
            <li>
              <div class="meta">
                <span class="status s-{p.status.toLowerCase()}">{statusText(p.status)}</span>
                <span class="t">{when(p.createdAt)}</span>
                {#if p.resolvedBy}<span class="by">· <bdi>{p.resolvedBy}</bdi></span>{/if}
              </div>
              {#if p.text}<p class="msg" dir="auto">{p.text}</p>{/if}
              {#if p.note}<p class="note" dir="auto">{p.note}</p>{/if}
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  </div>

  <footer>
    <textarea
      class="field"
      rows="2"
      maxlength="500"
      placeholder="Note to them (optional)"
      aria-label="Note for {r.display || r.login}"
      bind:value={note}
      disabled={busy}
    ></textarea>
    <div class="row">
      <Button small variant="primary" disabled={busy} onclick={() => onresolve(true, note)}>Approve</Button>
      <Button small variant="danger" disabled={busy} onclick={() => onresolve(false, note)}>Deny</Button>
    </div>
  </footer>
</div>

<style>
  .detail {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .who {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .name,
  .login {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .login {
    color: var(--muted);
    font-size: 0.78rem;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  h4 {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin: 0;
    color: var(--dim);
    font-size: 0.68rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  h4 .t {
    text-transform: none;
    letter-spacing: 0;
    font-weight: 600;
  }
  .t {
    color: var(--muted);
    font-size: 0.74rem;
    font-variant-numeric: tabular-nums;
  }
  .msg,
  .note {
    margin: 0;
    font-size: 0.86rem;
    overflow-wrap: anywhere;
    white-space: pre-wrap;
  }
  .note {
    color: var(--muted);
    font-size: 0.8rem;
  }
  .dim {
    margin: 0;
    color: var(--dim);
    font-size: 0.82rem;
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 2px 10px;
    margin: 0;
    font-size: 0.84rem;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .facts :global(ul) {
    padding: 0;
  }
  .counts {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    color: var(--muted);
    font-size: 0.82rem;
  }
  .counts b {
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .box {
    display: flex;
    flex-direction: column;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .msgs {
    height: 220px;
  }
  .comments {
    max-height: 240px;
  }
  .past {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .past li {
    padding: 6px 8px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
    font-size: 0.78rem;
  }
  .status {
    font-weight: 600;
  }
  .s-approved {
    color: var(--green);
  }
  .s-denied {
    color: var(--red);
  }
  .by {
    color: var(--muted);
    min-width: 0;
    overflow-wrap: anywhere;
  }
  footer {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 12px;
    border-top: 1px solid var(--border);
  }
  textarea {
    width: 100%;
    resize: vertical;
    min-height: 34px;
    font: inherit;
    font-size: 0.82rem;
  }
  .row {
    display: flex;
    gap: 6px;
  }
</style>
