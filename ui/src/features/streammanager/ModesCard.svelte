<!-- 1.35 Chat settings for your own chat: subscribers only, emote only, followers only with
     how long they must have followed, slow mode and unique messages. -->
<script lang="ts">
  import Toggle from '../../components/Toggle.svelte';
  import type { ChatModes } from '../../ipc/bindings/ChatModes';
  import type { SmMode } from '../../ipc/bindings/SmMode';
  import { smIpc } from '../../ipc/streammanager';
  import { toast } from '../../lib/toast.svelte';
  import Card from './Card.svelte';

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
    .catch((e) => toast.error(String(e)));

  function set(mode: SmMode) {
    busy = true;
    smIpc
      .setMode(mode)
      .then((m) => (modes = m))
      .catch((e) => toast.error(String(e)))
      .finally(() => (busy = false));
  }

  function followChange(e: Event) {
    followFor = Number((e.currentTarget as HTMLSelectElement).value);
    if (modes?.followerMode) set({ kind: 'followers', minutes: followFor });
  }

  function slowChange(e: Event) {
    slowFor = Number((e.currentTarget as HTMLSelectElement).value);
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
      <select class="field" value={followFor} disabled={busy} onchange={followChange} aria-label="Followed for at least">
        {#each FOLLOW as f (f.value)}<option value={f.value}>{f.label}</option>{/each}
      </select>
      <Toggle
        label="Followers only"
        checked={modes.followerMode}
        disabled={busy}
        onchange={(on) => set({ kind: 'followers', minutes: on ? followFor : null })}
      />
    </div>
    <div class="row">
      <span>Slow mode</span>
      <select class="field" value={slowFor} disabled={busy} onchange={slowChange} aria-label="Seconds between messages">
        {#each SLOW as s (s)}<option value={s}>{s}s</option>{/each}
      </select>
      <Toggle label="Slow mode" checked={modes.slowMode} disabled={busy} onchange={(on) => set({ kind: 'slow', seconds: on ? slowFor : null })} />
    </div>
    <div class="row">
      <span>Unique messages</span>
      <Toggle label="Unique messages" checked={modes.uniqueChatMode} disabled={busy} onchange={(on) => set({ kind: 'unique', on })} />
    </div>
  {:else}
    <p class="note">Loading…</p>
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
  select.field {
    height: 26px;
    padding: 0 6px;
    font-size: 0.8rem;
  }
</style>
