<!-- 1.26 Whispers: every conversation with its history on the left (search, start a new
     one, archived ones folded away), the open one on the right. In the main window or
     popped out. -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import { openMenuAt } from '../../components/menu.svelte';
  import type { WhisperThread } from '../../ipc/bindings/WhisperThread';
  import { whispersIpc } from '../../ipc/whispers';
  import { ago } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import Conversation from './Conversation.svelte';
  import { whispers } from './whispers.svelte';
  import { errorText } from '../../lib/errors';
  import Loading from '../../components/Loading.svelte';
  import SearchField from '../../components/SearchField.svelte';

  interface Props {
    popout?: boolean;
    /** `whispers` or `whispers/<login>` in a pop-out. */
    route?: string;
    onpopout?: () => void;
    onsignin?: () => void;
  }

  let { popout = false, route = '', onpopout, onsignin }: Props = $props();

  // svelte-ignore state_referenced_locally
  let selected = $state<string | null>(route.split('/')[1] || whispers.wanted);
  let query = $state('');
  let archived = $state(false);

  const s = $derived(whispers.state);
  const q = $derived(query.trim().toLowerCase().replace(/^@/, ''));
  const shown = $derived(
    s.threads.filter(
      (t) =>
        t.archived === archived &&
        (!q || t.other.login.includes(q) || t.other.name.toLowerCase().includes(q)),
    ),
  );
  const archivedCount = $derived(s.threads.filter((t) => t.archived).length);
  /** A new conversation: what's typed is a login nobody's in the list with. */
  const fresh = $derived(
    /^[a-z0-9_]{2,25}$/.test(q) && !s.threads.some((t) => t.other.login === q) ? q : null,
  );

  $effect(() => {
    void whispers.start();
  });

  // Someone asked for a conversation (a name's menu, a toast, an inline line).
  $effect(() => {
    if (whispers.asks && whispers.wanted) selected = whispers.wanted;
  });

  function pick(login: string) {
    selected = login;
    query = '';
  }

  function more(e: MouseEvent) {
    openMenuAt(e, [
      {
        label: 'Mark all read',
        icon: 'checkCheck',
        disabled: !s.unread,
        run: () => whispersIpc.readAll().catch((err) => toast.error(errorText(err))),
      },
      {
        label: 'Block whispers from strangers',
        icon: 'lock',
        note: 'Only people you follow or whispered can whisper you.',
        checked: s.blockStrangers,
        disabled: !s.history,
        run: () => whispersIpc.blockStrangers(!s.blockStrangers).catch((err) => toast.error(errorText(err))),
      },
      ...(!popout && onpopout ? ['separator' as const, { label: 'Pop out', icon: 'popout' as const, run: onpopout }] : []),
    ]);
  }

  function onscroll(e: Event) {
    const el = e.currentTarget as HTMLElement;
    if (s.more && !s.loading && el.scrollHeight - el.scrollTop - el.clientHeight < 80) void whispersIpc.more();
  }

  const preview = (t: WhisperThread) => (t.lastOwn ? `You: ${t.lastText}` : t.lastText);
</script>

{#if !s.account && whispers.failed}
  <div class="none">
    <EmptyState icon="whisper" text="Couldn’t load whispers. {whispers.failed}" />
    <Button variant="primary" onclick={() => void whispers.start()}>Retry</Button>
  </div>
{:else if !s.account}
  <div class="none">
    <EmptyState icon="whisper" text="Sign in to whisper. With a GQL sign-in, your past whispers load too." />
    {#if onsignin}<Button variant="primary" icon="login" onclick={onsignin}>Sign in</Button>{/if}
  </div>
{:else}
  <div class="whispers">
    <aside>
      <div class="top">
        <div class="search"><SearchField bind:value={query} label="Search or start a conversation" /></div>
        <IconButton icon="more" label="Options" hint="Mark all read, and who may whisper you." onclick={more} />
      </div>
      <div class="threads" {onscroll}>
        {#if fresh}
          <button class="thread new" onclick={() => pick(fresh)}>
            <span class="plus"><Icon name="plus" size={16} /></span>
            <span class="who"><span class="name">Whisper {fresh}</span></span>
          </button>
        {/if}
        {#each shown as t (t.id)}
          <button
            class="thread"
            class:on={selected === t.other.login}
            class:unread={t.unread > 0}
            style="--cd:{t.other.color.dark};--cl:{t.other.color.light}"
            onclick={() => pick(t.other.login)}
          >
            <Avatar src={t.other.avatar} name={t.other.name} size={32} />
            <span class="who">
              <span class="line1">
                <span class="name">{t.other.name}</span>
                {#if t.muted}<Icon name="bellOff" size={12} />{/if}
                <span class="when">{t.lastTs ? ago(t.lastTs) : ''}</span>
              </span>
              <span class="line2">
                <span class="last">{preview(t)}</span>
                {#if t.unread}<span class="badge">{t.unread > 99 ? '99+' : t.unread}</span>{/if}
              </span>
            </span>
          </button>
        {:else}
          {#if !fresh}{#if s.loading}<Loading rows={4} />{:else}<p class="empty">{q ? 'Nobody by that name.' : 'No whispers yet.'}</p>{/if}{/if}
        {/each}
        {#if archivedCount || archived}
          <button class="fold" onclick={() => (archived = !archived)}>
            {archived ? 'Back to conversations' : `Archived (${archivedCount})`}
          </button>
        {/if}
      </div>
    </aside>
    <section>
      {#if selected}
        {#key selected}
          <Conversation login={selected} account={s.account.id} history={s.history} />
        {/key}
      {:else}
        <EmptyState icon="whisper" text="Pick a conversation, or type a name to start one." />
      {/if}
    </section>
  </div>
{/if}

<style>
  .none {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
  }
  .whispers {
    flex: 1;
    display: grid;
    grid-template-columns: minmax(220px, 300px) 1fr;
    min-height: 0;
    min-width: 0;
  }
  aside {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-right: 1px solid var(--border);
    background: var(--page);
  }
  .top {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 8px;
  }
  .search {
    flex: 1;
    min-width: 0;
  }
  .threads {
    flex: 1;
    overflow-y: auto;
    padding: 0 6px 8px;
  }
  .thread {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 7px 8px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    text-align: left;
  }
  .thread:hover {
    background: var(--surface-hover);
  }
  .thread.on {
    background: var(--accent-dim);
  }
  .plus {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: var(--surface);
    color: var(--accent-text);
  }
  .who {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }
  .line1,
  .line2 {
    display: flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--cd, var(--text));
    font-weight: 600;
  }
  :global(:root[data-theme='light']) .thread .name {
    color: var(--cl, var(--text));
  }
  .new .name {
    color: var(--accent-text);
  }
  .when {
    margin-left: auto;
    flex: none;
    color: var(--dim);
    font-size: 11px;
  }
  .last {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--dim);
    font-size: 12px;
  }
  .unread .last {
    color: var(--text);
  }
  .badge {
    flex: none;
    min-width: 17px;
    padding: 0 5px;
    border-radius: 9px;
    background: var(--accent);
    color: var(--accent-ink);
    font-size: 10px;
    font-weight: 700;
    line-height: 17px;
    text-align: center;
  }
  .empty {
    margin: 20px 0;
    color: var(--dim);
    text-align: center;
  }
  .fold {
    width: 100%;
    margin-top: 6px;
    padding: 6px;
    background: none;
    border: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .fold:hover {
    color: var(--text);
  }
  section {
    display: flex;
    min-width: 0;
    min-height: 0;
    background: var(--page);
  }
</style>
