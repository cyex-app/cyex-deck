<!-- 1.49 The usercard's mod bar, shown when your account moderates this channel (1.57), in
     the 7TV extension's order (V101-05): ban or unban, warn, your timeout lengths (2.11),
     and the rest behind ⋯. Says when they're banned or timed out right now. V107-M8 For the
     broadcaster and lead mods, a Roles pair under it (like technorino's mod/VIP buttons):
     Mod or Unmod and VIP or Unvip from what they hold now. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import { openMenu } from '../../components/menu.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { BanStatus } from '../../ipc/bindings/BanStatus';
  import type { Profile } from '../../ipc/bindings/Profile';
  import { modIpc } from '../../ipc/moderation';
  import { usercardIpc } from '../../ipc/rules';
  import { everyShown } from '../../lib/awake';
  import { formatDate, formatTime } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { FAIL, QUICK } from './notices';
  import { mods, timeouts, treatItems } from '../modview/moderation.svelte';
  import { timeLeft } from './countdown';
  import { MOD_ROLE_NOTE } from '../modview/lineMod';
  import { afterAction } from './optimistic';
  import { accounts } from '../../lib/accounts.svelte';
  import { standingOf, type Standing } from '../modview/standing';
  import { standings } from '../modview/standings.svelte';
  import { errorText } from '../../lib/errors';
  import type { VipSlots } from '../../ipc/bindings/VipSlots';
  import { rolesIpc } from '../../ipc/roles';

  interface Props {
    profile: Profile;
    channel: string;
    account: string;
    /** From the mod logs; `undefined` while unknown (no GQL sign-in). */
    ban: BanStatus | null | undefined;
    /** After an action, so the card reads the logs again. V107-R45 `next` is the ban state the
     *  action leaves (`null` none), shown at once. */
    onchange: (next?: BanStatus | null) => void;
    /** M12 Mod or VIP, from their last line's badges. */
    is?: Standing;
  }

  let { profile, channel, account, ban, onchange, is }: Props = $props();

  const role = $derived(mods.role(account, channel));
  const caps = $derived(mods.caps(account, channel));
  /** V107-R44 The broadcaster acting on a mod: Twitch also takes the mod role. */
  const roleNote = $derived(is?.mod && caps?.broadcaster ? `. ${MOD_ROLE_NOTE}` : '');
  // V107-M8 Mod and VIP changes are the broadcaster's and lead mods' (1.50, `offered`).
  const lead = $derived(caps?.roles ?? false);
  /** What Twitch says they hold here (public badges), over their last line's. */
  let looked = $state<Standing | undefined>();
  $effect(() => {
    if (!lead) return;
    let live = true;
    usercardIpc
      .badges(profile.login, channel)
      .then((list) => live && (looked = standingOf(list)))
      .catch(() => {});
    return () => (live = false);
  });
  const holds = $derived(standings.of(channel, profile.login, looked ?? is));
  // V107-R36 Your own channel's VIP slots: VIP is off when they're all taken or locked.
  // V107-R42 Rust says why (`why`) and the count (`label`).
  let slots = $state<VipSlots | null>(null);
  let slotsAsk = $state(0);
  $effect(() => {
    void slotsAsk;
    if (!lead) return;
    let live = true;
    rolesIpc
      .vipSlots(account, channel)
      .then((s) => live && (slots = s))
      .catch(() => {});
    return () => (live = false);
  });
  const vipOff = $derived(holds?.vip ? null : (slots?.why ?? null));
  let busy = $state(false);
  let warning = $state(false);
  let reason = $state('');

  async function run(name: string, args: string) {
    busy = true;
    try {
      const line = await modIpc.run(account, channel, name, args);
      if (line) toast.info(line, QUICK);
      onchange(afterAction(name, args, Date.now(), accounts.list.find((a) => a.userId === account)?.displayName ?? null));
      // Only these change who holds a VIP slot.
      if (['vip', 'unvip', 'mod', 'unmod'].includes(name)) slotsAsk++;
    } catch (e) {
      toast.error(errorText(e), FAIL);
    } finally {
      busy = false;
    }
  }

  async function warn() {
    const why = reason.trim();
    if (!why) return;
    await run('warn', `${profile.login} ${why}`);
    reason = '';
    warning = false;
  }

  function more(e: MouseEvent) {
    if (!caps) return;
    const line = (text: string) => {
      const [name, ...rest] = text.slice(1).split(' ');
      void run(name, rest.join(' '));
    };
    openMenu(
      e.currentTarget as HTMLElement,
      treatItems(caps, profile.login, line, holds, false),
      'end',
    );
  }

  // V107-R24 Ticks each second only while a timeout runs; the card being open is this component living.
  let now = $state(Date.now());
  const endsAt = $derived(ban?.until ? new Date(ban.until).getTime() : 0);
  $effect(() => {
    if (!endsAt || endsAt <= Date.now()) return;
    now = Date.now();
    // V107-H1 Not while the card is hidden; once when it shows.
    const stop = everyShown(() => {
      now = Date.now();
      if (now >= endsAt) {
        stop();
        onchange(null);
      }
    }, 1000);
    return stop;
  });
  const banned = $derived(!!ban && (!ban.until || endsAt > now));
  const left = $derived(endsAt ? timeLeft(endsAt, now) : null);

  const until = $derived.by(() => {
    if (!ban?.until) return '';
    const d = new Date(ban.until);
    const today = formatDate(d) === formatDate(new Date());
    return today ? formatTime(d, settings.v.chat.timestamps) : formatDate(d);
  });
</script>

{#if role}
  <section>
    <div class="bar">
      <!-- V107-R45 Both stay put; Unban only works where there is something to lift. -->
      <button class="act bad" disabled={busy} onclick={() => run('ban', profile.login)} use:tooltip={`Ban${roleNote}`}>
        <Icon name="gavel" size={15} />
      </button>
      <button
        class="act ok"
        disabled={busy || !banned}
        onclick={() => run('unban', profile.login)}
        use:tooltip={banned ? 'Unban' : 'Not banned or timed out'}
      >
        <Icon name="undo" size={15} />
      </button>
      <button
        class="act warn"
        class:on={warning}
        disabled={busy}
        onclick={() => (warning = !warning)}
        use:tooltip={{ label: 'Warn', hint: 'They have to read it before they can chat again.' }}
      >
        <Icon name="alert" size={15} />
      </button>
      <div class="times">
        {#each timeouts() as t (t.secs)}
          <button disabled={busy} onclick={() => run('timeout', `${profile.login} ${t.secs}`)} use:tooltip={`Time out ${t.label}${roleNote}`}
            >{t.label}</button
          >
        {/each}
      </div>
      <IconButton icon="more" label="More" hint="Monitor, restrict." size={15} onclick={more} />
    </div>
    {#if lead}
      <div class="roles">
        <span class="label">Roles</span>
        <div class="seg" role="group" aria-label="Roles">
          <button
            class:held={holds?.mod}
            disabled={busy}
            onclick={() => run(holds?.mod ? 'unmod' : 'mod', profile.login)}
            use:tooltip={holds?.mod ? 'Takes their mod away.' : holds?.vip ? 'Removes their VIP first.' : 'Makes them a mod.'}
          >
            <Icon name="shield" size={13} />{holds?.mod ? 'Unmod' : 'Mod'}
          </button>
          <button
            class:held={holds?.vip}
            class:off={!!vipOff}
            disabled={busy}
            aria-disabled={!!vipOff}
            onclick={() => !vipOff && run(holds?.vip ? 'unvip' : 'vip', profile.login)}
            use:tooltip={{
              label: vipOff ?? (holds?.vip ? 'Takes their VIP away.' : holds?.mod ? 'Removes their mod first.' : 'Makes them a VIP.'),
              hint: slots?.label,
            }}
          >
            <Icon name="gem" size={13} />{holds?.vip ? 'Unvip' : 'VIP'}
          </button>
        </div>
      </div>
    {/if}
    {#if warning}
      <form
        class="reason"
        onsubmit={(e) => {
          e.preventDefault();
          void warn();
        }}
      >
        <!-- svelte-ignore a11y_autofocus -->
        <input class="field" maxlength="500" placeholder="Why? They’ll see this." bind:value={reason} autofocus />
        <button class="send" disabled={busy || !reason.trim()}>Warn</button>
      </form>
    {/if}
    {#if ban && (!ban.until || left)}
      <p class="state">
        {ban.until ? `Timed out until ${until}` : 'Banned'}{ban.by ? ` by ${ban.by}` : ''}{ban.until ? ` · ${left}` : ''}{ban.reason ? `: ${ban.reason}` : ''}
      </p>
    {/if}
  </section>
{/if}

<style>
  section {
    border-top: 1px solid var(--border);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 4px 8px;
  }
  .act {
    display: grid;
    place-items: center;
    flex: none;
    width: 28px;
    height: 26px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
  }
  .act:hover,
  .act.on {
    background: var(--row-hover);
  }
  .bad:hover {
    color: var(--red);
  }
  .ok:hover:not(:disabled) {
    color: var(--green);
  }
  .act:disabled {
    opacity: 0.4;
  }
  .act:disabled:hover {
    background: none;
  }
  .warn:hover,
  .warn.on {
    color: var(--yellow);
  }
  .times {
    display: flex;
    flex: 1;
    flex-wrap: wrap;
    gap: 2px;
    min-width: 0;
  }
  .times button {
    flex: 1;
    min-width: 30px;
    height: 24px;
    padding: 0 4px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.75rem;
    font-weight: 600;
  }
  .times button:hover {
    background: var(--row-hover);
    color: var(--text);
  }
  .roles {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 8px 6px 14px;
  }
  .label {
    color: var(--muted);
    font-size: 0.75rem;
  }
  .seg {
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .seg button {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 24px;
    padding: 0 10px;
    background: none;
    border: 0;
    color: var(--muted);
    font-size: 0.75rem;
    font-weight: 600;
  }
  .seg button + button {
    border-left: 1px solid var(--border);
  }
  .seg button:hover:not(:disabled) {
    background: var(--row-hover);
    color: var(--text);
  }
  .seg button.held {
    color: var(--text);
  }
  .seg button.off {
    opacity: 0.5;
    cursor: default;
  }
  .seg button.off:hover {
    background: none;
    color: var(--muted);
  }
  .seg button.held:hover:not(:disabled) {
    color: var(--red);
  }
  .reason {
    display: flex;
    gap: 6px;
    padding: 0 8px 6px;
  }
  .reason .field {
    flex: 1;
    min-width: 0;
  }
  .send {
    padding: 0 10px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--text);
    font-size: 0.8rem;
  }
  .state {
    margin: 0;
    padding: 0 14px 6px;
    color: var(--red);
    font-size: 0.8rem;
    /* V107-R24 The countdown ticks without nudging the words after it. */
    font-variant-numeric: tabular-nums;
  }
</style>
