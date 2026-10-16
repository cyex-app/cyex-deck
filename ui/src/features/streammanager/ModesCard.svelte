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
  import Select from '../../components/Select.svelte';
  import { choicesOf } from '../../components/select.ts';
  import Loading from '../../components/Loading.svelte';

  /** Minimum follow age, in minutes. 0 = any follower. */
  const FOLLOW: { value: number; label: string }[] = [
    { value: 0, label: 'Any follower' },
    { value: 10, label: '10 minutes' },
    { value: 30, label: '30 minutes' },
    { value: 60, label: '1 hour' },
    { value: 1440, label: '1 day' },
    { value: 10080, label: '1 week' },
    { value: 43200, label: '1 month' },
    { value: 129600, label: '3 months' },
  ];
  const SLOW = [3, 5, 10, 20, 30, 60, 120];

  let modes = $state<ChatModes | null>(null);
  let busy = $state(false);
  let followFor = $state(10);
  let slowFor = $state(30);

  smIpc
    .modes()
    .then((m) => {
      modes = m;
      if (m.followerModeDuration != null) followFor = m.followerModeDuration;
      if (m.slowModeWaitTime != null) slowFor = m.slowModeWaitTime;
    })
    .catch((e) => toast.error(errorText(e)));

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
      <Select label="Followed for at least" width="140px" value={String(followFor)} disabled={busy} options={choicesOf(FOLLOW)} onchange={followChange} />
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
        options={SLOW.map((s) => ({ value: String(s), label: `${s}s` }))}
        onchange={slowChange}
      />
      <Toggle label="Slow mode" checked={modes.slowMode} disabled={busy} onchange={(on) => set({ kind: 'slow', seconds: on ? slowFor : null })} />
    </div>
    <div class="row">
      <span>Unique messages</span>
      <Toggle label="Unique messages" checked={modes.uniqueChatMode} disabled={busy} onchange={(on) => set({ kind: 'unique', on })} />
    </div>
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
