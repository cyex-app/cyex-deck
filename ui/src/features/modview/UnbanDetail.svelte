<!-- V108-C36 One unban request in full. V108-C36b Laid out as Twitch's mod view does it: who
     asks (picture, name, account age) with an (i) for more; the ban in a row of short facts
     (who, when, until, bans, timeouts) and its reason; then tabs for the chat log (it takes
     every pixel left, ends with the ban as its last line), mod comments and, when there are
     any, earlier requests; at the bottom their message as a quote, then Deny or Unban. The
     note field shows only once one is picked (and focus moves into it); Confirm sends it and
     the next request opens. Popped out and wide the ban row always shows and (i) adds the
     rest (follow, sub, messages, warnings); narrow, (i) shows it all. The eye hides who
     banned them in the facts and in the chat log's ban line. -->
<script lang="ts">
  import { tick, untrack } from 'svelte';

  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';
  import Icon from '../../components/Icon.svelte';
  import type { IconName } from '../../components/icons';
  import IconButton from '../../components/IconButton.svelte';
  import Loading from '../../components/Loading.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
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
  import { banParts, bannedBy, commentCount, followCell, messageCount, statusText, subCell } from './unban';
  import { usercardIpc } from '../../ipc/rules';
  import type { FollowAge } from '../../ipc/bindings/FollowAge';
  import type { SubStatus } from '../../ipc/bindings/SubStatus';
  import { radioKey } from '../../components/select';

  interface Props {
    account: string;
    /** The channel. */
    login: string;
    request: UnbanRequest;
    /** Who banned them shows as "Hidden". */
    hideBy: boolean;
    busy: boolean;
    /** Narrow: the request stands alone with a Back button, and (i) holds every fact. */
    stacked: boolean;
    /** (i): the extra facts show (kept by the list while switching requests). */
    more: boolean;
    onmore: (on: boolean) => void;
    onback: () => void;
    onresolve: (approve: boolean, note: string) => void;
  }

  let { account, login, request, hideBy, busy, stacked, more, onmore, onback, onresolve }: Props = $props();

  // One request for the component's life (the list keys it).
  // svelte-ignore state_referenced_locally
  const r = request;
  const name = r.display || r.login;
  const uid = $props.id();

  let profile = $state<Profile | null | undefined>(undefined);
  let history = $state<ModHistory | null | undefined>(undefined);
  let historyError = $state('');
  let past = $state.raw<PastUnban[]>([]);
  /** V108-C36g One note per choice: a deny reason doesn't turn into an unban note. */
  const notes = $state({ unban: '', deny: '' });
  let decision = $state<'unban' | 'deny' | null>(null);
  let tab = $state<'chat' | 'comments' | 'earlier'>('chat');
  let noteBox = $state<HTMLTextAreaElement>();
  let tabBar = $state<HTMLElement>();

  /** V108-C36e Follow age and sub sit in the facts row, not only behind (i). */
  let follow = $state<FollowAge | null>(null);
  let sub = $state<SubStatus | null>(null);
  untrack(() => {
    usercardIpc
      .followAge(login, r.userId)
      .then((f) => (follow = f))
      .catch(() => {});
    usercardIpc
      .subStatus(r.login, login)
      .then((s) => (sub = s))
      .catch(() => {});
  });
  const followText = $derived(followCell(follow, new Date()));
  const subText = $derived(subCell(sub));

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
  const showBan = $derived(!stacked || more);
  const tabs = $derived.by(() => {
    const out: { id: typeof tab; label: string; icon: IconName; count: string }[] = [
      { id: 'chat', label: 'Chat logs', icon: 'chat', count: history ? messageCount(history.messageCount) : '' },
      { id: 'comments', label: 'Mod comments', icon: 'pencil', count: history ? commentCount(history.comments) : '' },
    ];
    if (past.length) out.push({ id: 'earlier', label: 'Earlier requests', icon: 'history', count: formatNumber(past.length) });
    return out;
  });

  function onTabKey(e: KeyboardEvent) {
    const at = tabs.findIndex((t) => t.id === tab);
    const to = radioKey(e, at, tabs.length);
    if (to === null) return;
    e.preventDefault();
    tab = tabs[to].id;
    (tabBar?.querySelectorAll<HTMLElement>('[role="tab"]')[to])?.focus();
  }

  /** Picking one shows the note field and moves into it; picking it again takes it back. */
  async function choose(d: 'unban' | 'deny') {
    decision = decision === d ? null : d;
    if (!decision) return;
    await tick();
    noteBox?.focus({ preventScroll: true });
  }

  function confirm() {
    if (!decision || busy) return;
    onresolve(decision === 'unban', notes[decision]);
  }

  function onNoteKey(e: KeyboardEvent) {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      confirm();
    } else if (e.key === 'Escape' && !e.isComposing) {
      // Back to the choice, field closed; the notes stay for a second pick.
      e.preventDefault();
      e.stopPropagation();
      const was = decision;
      decision = null;
      document.getElementById(`${uid}-${was}`)?.focus();
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (stacked && e.key === 'Escape' && !e.defaultPrevented && !e.isComposing) {
      e.preventDefault();
      onback();
    }
  }
</script>

{#snippet banTail()}
  {#if ban}
    <p class="banline">
      <Icon name="ban" size={13} />
      <span class="what">{#each banParts(ban, name, hideBy) as part}{#if part.name}<bdi>{part.text}</bdi>{:else}{part.text}{/if}{/each}{ban.until ? ` until ${when(ban.until)}` : ''}</span>
      {#if ban.since}<span class="t">{when(ban.since)}</span>{/if}
    </p>
  {/if}
{/snippet}

<!-- svelte-ignore a11y_no_static_element_interactions (Escape goes back from anywhere inside) -->
<div class="detail" {onkeydown}>
  <header>
    {#if stacked}<IconButton icon="chevronLeft" label="Back to requests" size={16} onclick={onback} />{/if}
    <Avatar src={profile?.avatar} name={name} size={36} />
    <div class="who">
      <span class="nameline">
        <b class="name"><bdi>{name}</bdi></b>
        {#if r.display && r.display.toLowerCase() !== r.login}<span class="login">{r.login}</span>{/if}
      </span>
      <span class="created">
        <Icon name="cake" size={12} />
        {#if profile?.createdAt}Account created {formatDate(new Date(profile.createdAt))}{:else if profile === undefined}&nbsp;{:else}Account age unknown{/if}
      </span>
    </div>
    <IconButton
      icon="info"
      label={more ? 'Fewer details' : 'More details'}
      hint="Messages, warnings, followers and earlier names."
      size={16}
      active={more}
      toggle
      onclick={() => onmore(!more)}
    />
  </header>

  {#if showBan || more}
    <div class="facts">
      {#if showBan}
        {#if history === undefined}
          <Loading rows={1} />
        {:else if history === null}
          <p class="dim">{historyError || 'Couldn’t load their mod history.'} <Button small variant="link" onclick={readHistory}>Retry</Button></p>
        {:else}
          <dl class="cols">
            {#if ban}
              <div class="by">
                <dt>{ban.until ? 'Timed out by' : 'Banned by'}</dt>
                <dd class:dim={hideBy || !ban.by?.trim()}><bdi>{bannedBy(ban.by, hideBy)}</bdi></dd>
              </div>
              {#if ban.since}<div><dt>{ban.until ? 'Timed out' : 'Banned'}</dt><dd class="t">{when(ban.since)}</dd></div>{/if}
              {#if ban.until}<div><dt>Until</dt><dd class="t">{when(ban.until)}</dd></div>{/if}
            {:else}
              <div><dt>Ban</dt><dd class="dim">Not banned now</dd></div>
            {/if}
            <div><dt>Bans</dt><dd class="t">{formatNumber(history.bans)}</dd></div>
            <div><dt>Timeouts</dt><dd class="t">{formatNumber(history.timeouts)}</dd></div>
            {#if followText}<div><dt>Following</dt><dd class:dim={followText === 'Not following'}>{followText}</dd></div>{/if}
            {#if subText}<div><dt>Sub</dt><dd class:dim={subText === 'Never' || subText === 'Hidden'}>{subText}</dd></div>{/if}
          </dl>
          {#if ban}
            <p class="reason" dir="auto" use:tooltip={ban.reason.length > 90 ? ban.reason : ''}>
              <span class="lbl">Reason</span>
              {#if ban.reason}{ban.reason}{:else}<span class="dim">None given</span>{/if}
            </p>
          {/if}
        {/if}
      {/if}
      {#if more}
        {#if history}
          <dl class="cols">
            <div><dt>Messages</dt><dd class="t">{messageCount(history.messageCount)}</dd></div>
            <div><dt>Warnings</dt><dd class="t">{formatNumber(history.warnings)}</dd></div>
          </dl>
        {/if}
        {#if profile}
          <div class="extra">{#key profile.id}<Facts {profile} channel={login} />{/key}</div>
        {/if}
      {/if}
    </div>
  {/if}

  <div class="tabs" role="tablist" aria-label="About {name}" bind:this={tabBar}>
    {#each tabs as t (t.id)}
      <button
        role="tab"
        id="{uid}-tab-{t.id}"
        aria-selected={tab === t.id}
        aria-controls="{uid}-pane"
        tabindex={tab === t.id ? 0 : -1}
        class:on={tab === t.id}
        onclick={() => (tab = t.id)}
        onkeydown={onTabKey}
        use:tooltip={{ label: t.label }}
      >
        <Icon name={t.icon} size={14} />
        <span class="tl">{t.label}</span>
        {#if t.count}<span class="n">{t.count}</span>{/if}
      </button>
    {/each}
  </div>

  <div class="pane" id="{uid}-pane" role="tabpanel" aria-labelledby="{uid}-tab-{tab}">
    {#if tab === 'chat'}
      <Timeline
        channel={login}
        userId={r.userId}
        login={r.login}
        {name}
        live={[]}
        modAccount={account}
        {account}
        tail={ban ? banTail : undefined}
      />
    {:else if tab === 'comments'}
      {#if history}
        <Comments comments={history.comments} channel={login} {account} targetId={r.userId} onchange={readHistory} />
      {:else}
        <p class="dim pad">{history === null ? 'Couldn’t load comments.' : 'Loading…'}</p>
      {/if}
    {:else}
      <ul class="past">
        {#each past as p (p.id)}
          <li>
            <div class="meta">
              <span class="status s-{p.status.toLowerCase()}">{statusText(p.status)}</span>
              <span class="t">{when(p.createdAt)}</span>
              {#if p.resolvedBy}<span class="rby">· <bdi>{p.resolvedBy}</bdi></span>{/if}
            </div>
            {#if p.text}<p class="msg" dir="auto">{p.text}</p>{:else}<p class="dim">No message.</p>{/if}
            {#if p.note}<p class="note" dir="auto">{p.note}</p>{/if}
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <footer>
    <blockquote>
      <span class="asked">Their request <span class="t">{when(r.createdAt)}</span></span>
      {#if r.text}<p class="msg" dir="auto">{r.text}</p>{:else}<p class="dim">No message.</p>{/if}
    </blockquote>

    <div class="reveal" class:open={!!decision} inert={!decision}>
      <div>
        <textarea
          class="field"
          bind:this={noteBox}
          rows="2"
          maxlength="500"
          placeholder={decision === 'deny' ? 'Why not? They’ll see this (optional)' : 'A note they’ll see (optional)'}
          aria-label="Note for {name}"
          bind:value={notes[decision ?? 'unban']}
          disabled={busy}
          onkeydown={onNoteKey}
        ></textarea>
      </div>
    </div>

    <div class="act">
      <button
        id="{uid}-deny"
        class="pick deny"
        class:on={decision === 'deny'}
        aria-pressed={decision === 'deny'}
        disabled={busy}
        onclick={() => choose('deny')}
      >
        <Icon name={decision === 'deny' ? 'check' : 'x'} size={14} />Deny
      </button>
      <button
        id="{uid}-unban"
        class="pick unban"
        class:on={decision === 'unban'}
        aria-pressed={decision === 'unban'}
        disabled={busy}
        onclick={() => choose('unban')}
      >
        <Icon name={decision === 'unban' ? 'check' : 'lockOpen'} size={14} />Unban
      </button>
      <button
        class="go"
        disabled={!decision || busy}
        onclick={confirm}
        use:tooltip={{ label: 'Confirm', hint: 'Sends it and opens the next request. Ctrl+Enter in the note.' }}
      >
        <span class="gl">{busy ? 'Sending…' : 'Confirm'}</span>
        <Icon name="chevronRight" size={15} />
      </button>
    </div>
  </footer>
</div>

<style>
  .detail {
    flex: 1;
    container: unban / inline-size;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    /* A very short window scrolls the whole request rather than crushing the log. */
    overflow-y: auto;
  }
  header {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    padding: 10px 8px 10px 12px;
  }
  .who {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .nameline {
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.95rem;
  }
  .login {
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--dim);
    font-size: var(--text-hint);
  }
  .created {
    display: flex;
    align-items: center;
    gap: 5px;
    color: var(--muted);
    font-size: var(--text-hint);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .created :global(svg) {
    flex: none;
    color: var(--dim);
  }

  /* The facts: label over value, columns that line up and wrap. */
  .facts {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 45%;
    overflow-y: auto;
    padding: 0 12px 10px;
  }
  .cols {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 24px;
    margin: 0;
  }
  .cols > div {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .cols .by {
    max-width: min(100%, 240px);
  }
  dt {
    color: var(--dim);
    font-size: var(--text-label);
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    white-space: nowrap;
  }
  dd {
    margin: 0;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.84rem;
    font-weight: 600;
  }
  .reason {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
    margin: 0;
    font-size: 0.84rem;
    overflow-wrap: anywhere;
  }
  .lbl {
    margin-right: 6px;
    color: var(--dim);
    font-size: var(--text-label);
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  /* Created is already under the name. */
  .extra :global(li:first-child) {
    display: none;
  }
  .extra :global(ul) {
    padding: 0;
  }
  .t {
    font-variant-numeric: tabular-nums;
  }
  .dim {
    margin: 0;
    color: var(--dim);
    font-weight: 400;
  }
  .pad {
    padding: 12px;
    font-size: 0.82rem;
  }

  /* Tabs: one hairline under them, the open one underlined in the text colour. */
  .tabs {
    flex: none;
    display: flex;
    gap: 2px;
    padding: 0 8px;
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
    overflow-x: auto;
    scrollbar-width: none;
  }
  .tabs button {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
    height: 34px;
    padding: 0 8px;
    background: none;
    border: 0;
    box-shadow: inset 0 -2px 0 transparent;
    color: var(--muted);
    font-size: 0.8rem;
    font-weight: 600;
    transition:
      color var(--fast) ease-out,
      box-shadow var(--fast) ease-out;
  }
  .tabs button:hover {
    color: var(--text);
  }
  .tabs button.on {
    color: var(--text);
    box-shadow: inset 0 -2px 0 var(--text);
  }
  .tabs button:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: -2px;
  }
  .n {
    color: var(--dim);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .on .n {
    color: var(--muted);
  }

  /* The log takes all the height that's left. */
  .pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 140px;
    background: var(--bg);
  }
  .banline {
    display: flex;
    align-items: baseline;
    gap: 6px;
    margin: 6px 0 2px;
    padding: 4px 14px;
    color: var(--red);
    font-size: 0.82rem;
    font-weight: 600;
  }
  .banline :global(svg) {
    flex: none;
    align-self: center;
  }
  .banline .what {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .banline .t {
    flex: none;
    color: var(--dim);
    font-size: 0.72rem;
  }
  .past {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    margin: 0;
    padding: 8px 12px;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .past li {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-bottom: 10px;
    border-bottom: 1px solid var(--border);
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
    font-size: var(--text-hint);
  }
  .status {
    font-weight: 700;
  }
  .s-approved {
    color: var(--green);
  }
  .s-denied {
    color: var(--red);
  }
  .rby {
    min-width: 0;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .msg,
  .note {
    margin: 0;
    font-size: 0.84rem;
    overflow-wrap: anywhere;
    white-space: pre-wrap;
  }
  .note {
    color: var(--muted);
  }

  /* The dock: their words, then the decision. */
  footer {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    border-top: 1px solid var(--border);
  }
  blockquote {
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 7.5em;
    overflow-y: auto;
    margin: 0;
    padding: 2px 0 2px 10px;
    box-shadow: inset 2px 0 0 var(--border-strong);
  }
  .asked {
    color: var(--dim);
    font-size: var(--text-hint);
  }
  .asked .t {
    margin-left: 4px;
  }
  blockquote .dim {
    font-size: 0.84rem;
  }

  /* The note opens under the quote, so Deny, Unban and Confirm stay where the pointer is. */
  .reveal {
    display: grid;
    grid-template-rows: 0fr;
    opacity: 0;
    margin-top: -8px;
    transition:
      grid-template-rows var(--base) var(--ease),
      opacity var(--base) ease-out,
      margin-top var(--base) var(--ease);
  }
  .reveal.open {
    grid-template-rows: 1fr;
    opacity: 1;
    margin-top: 0;
  }
  .reveal > div {
    min-height: 0;
    overflow: hidden;
    /* Room for the field's focus ring inside the clip. */
    padding: 2px;
    margin: -2px;
  }
  textarea {
    display: block;
    width: 100%;
    min-height: 52px;
    resize: vertical;
    font: inherit;
    font-size: 0.84rem;
  }
  .act {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .pick,
  .go {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 32px;
    padding: 0 12px;
    border-radius: var(--radius);
    font-size: 0.84rem;
    font-weight: 600;
    white-space: nowrap;
    transition:
      background var(--fast) ease-out,
      border-color var(--fast) ease-out,
      color var(--fast) ease-out;
  }
  .pick {
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    color: var(--text);
  }
  .pick:hover:not(:disabled) {
    background: var(--elevated-2);
  }
  .pick.deny.on {
    background: color-mix(in srgb, var(--red) 14%, transparent);
    border-color: var(--red);
    color: var(--red);
  }
  .pick.unban.on {
    background: var(--accent-dim);
    border-color: var(--accent-border);
    color: var(--accent-text);
  }
  .go {
    /* V108-C36g Confirm stays beside the choices, however wide the dock. */
    margin-left: 4px;
    background: var(--accent);
    border: 1px solid transparent;
    color: var(--accent-ink);
  }
  .go:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  .go:disabled {
    background: var(--elevated);
    border-color: var(--border);
    color: var(--dim);
  }
  .pick:active:not(:disabled),
  .go:active:not(:disabled) {
    transform: scale(0.97);
    transition: none;
  }
  .pick:focus-visible,
  .go:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
  .pick:disabled {
    opacity: 0.6;
  }

  /* Narrow: tabs and Confirm go to icons (their names stay in the tooltips). */
  @container unban (max-width: 420px) {
    .tl,
    .gl {
      display: none;
    }
    .go {
      width: 32px;
      padding: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .reveal {
      transition: none;
    }
  }
</style>
