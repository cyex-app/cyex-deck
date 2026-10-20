<!-- 1.35 Chat settings for your own chat: subscribers only, emote only, followers only with
     how long they must have followed, slow mode and unique messages. -->
<script lang="ts">
  import Toggle from '../../components/Toggle.svelte';
  import type { ChatModes } from '../../ipc/bindings/ChatModes';
  import type { SmMode } from '../../ipc/bindings/SmMode';
  import { smIpc } from '../../ipc/streammanager';
  import { toast } from '../../lib/toast.svelte';
  import Card from '../../components/PanelCard.svelte';
  import { errorText } from '../../lib/errors';
  import { formatSecs } from '../../lib/format';
  import Select from '../../components/Select.svelte';
  import { choicesOf } from '../../components/select.ts';
  import Loading from '../../components/Loading.svelte';
  import Button from '../../components/Button.svelte';

  /** Minimum follow age, in minutes. 0 = any follower. */
  const FOLLOW: { value: number; label: string }[] = [
    { value: 0, label: 'Any follower' },
    { value: 10, label: formatSecs(600) },
    { value: 30, label: formatSecs(1800) },
    { value: 60, label: formatSecs(3600) },
    { value: 1440, label: formatSecs(86_400) },
    { value: 10080, label: formatSecs(604_800) },
    { value: 43200, label: formatSecs(2_592_000) },
    { value: 129600, label: formatSecs(7_776_000) },
  ];
  const SLOW = [3, 5, 10, 20, 30, 60, 120];

  let modes = $state<ChatModes | null>(null);
  let failed = $state(false);
  let busy = $state(false);
  let followFor = $state(10);
  let slowFor = $state(30);

  // v1.07 T4: a failed load offers Try again instead of spinning forever.
  function load() {
    failed = false;
    smIpc
      .modes()
      .then((m) => {
        modes = m;
        if (m.followerModeDuration != null) followFor = m.followerModeDuration;
        if (m.slowModeWaitTime != null) slowFor = m.slowModeWaitTime;
      })
      .catch((e) => {
        failed = true;
        toast.error(errorText(e));
      });
  }
  load();

  // v1.07 T4: a length set elsewhere (/slow 45) shows as itself, not as a blank picker.
  const followOptions = $derived(
    FOLLOW.some((f) => f.value === followFor)
      ? FOLLOW
      : [...FOLLOW, { value: followFor, label: formatSecs(followFor * 60) }].sort((a, b) => a.value - b.value),
  );
  const slowOptions = $derived([...new Set([...SLOW, slowFor])].sort((a, b) => a - b));

  function set(mode: SmMode) {
    busy = true;
    smIpc
      .setMode(mode)
      .then((m) => (modes = m))
      .catch((e) => toast.error(errorText(e)))
      .finally(() => (busy = false));
  }

  function followChange(v: string) {
    followFor = Number(v);
    if (modes?.followerMode) set({ kind: 'followers', minutes: followFor });
  }

  function slowChange(v: string) {
    slowFor = Number(v);
    if (modes?.slowMode) set({ kind: 'slow', seconds: slowFor });
  }
</script>

<Card title="Chat settings">
  {#if modes}
    <div class="row">
      <span>Subscribers only</span>
      <Toggle label="Subscribers only" checked={modes.subscriberMode} disabled={busy} onchange={(on) => set({ kind: 'subs', on })} />
    </div>
    <div class="row">
      <span>Emote only</span>
      <Toggle label="Emote only" checked={modes.emoteMode} disabled={busy} onchange={(on) => set({ kind: 'emote', on })} />
    </div>
    <div class="row">
      <span>Followers only</span>
      <Select label="Followed for at least" width="140px" value={String(followFor)} disabled={busy} options={choicesOf(followOptions)} onchange={followChange} />
      <Toggle
        label="Followers only"
        checked={modes.followerMode}
        disabled={busy}
        onchange={(on) => set({ kind: 'followers', minutes: on ? followFor : null })}
      />
    </div>
    <div class="row">
      <span>Slow mode</span>
      <Select
        label="Seconds between messages"
        width="140px"
        value={String(slowFor)}
        disabled={busy}
        options={slowOptions.map((s) => ({ value: String(s), label: formatSecs(s) }))}
        onchange={slowChange}
      />
      <Toggle label="Slow mode" checked={modes.slowMode} disabled={busy} onchange={(on) => set({ kind: 'slow', seconds: on ? slowFor : null })} />
    </div>
    <div class="row">
      <span>Unique messages</span>
      <Toggle label="Unique messages" checked={modes.uniqueChatMode} disabled={busy} onchange={(on) => set({ kind: 'unique', on })} />
    </div>
  {:else if failed}
    <p class="failed">Couldn’t load the chat settings. <Button small variant="link" onclick={load}>Try again</Button></p>
  {:else}
    <Loading />
  {/if}
</Card>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 30px;
    /* A short card scrolls; a wrapped row keeps its two lines instead of overlapping. */
    flex: none;
  }
  .failed {
    margin: 0;
    color: var(--muted);
    font-size: 0.85rem;
  }
  .row span {
    flex: 1;
    font-size: 0.85rem;
  }
  /* Narrow: the name and its switch share the first line, the picker gets the next. */
  @container (max-width: 320px) {
    .row > :global(.select) {
      order: 1;
    }
  }
</style>
