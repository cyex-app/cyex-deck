<!-- V101-05 The usercard's head, after the 7TV extension's: banner behind, picture, name in
     its color or 7TV paint, the paint's name, roles and every badge they own (V102-13), and
     small buttons for the note, the logs, the 7TV profile (V108-C16) and the Twitch profile. V101-23 switches the 7TV
     picture for Twitch's. V102-25 A flag when Twitch hides the account. V108-C09 Streamer mode
     blurs the picture until it is clicked. -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { Badge } from '../../ipc/bindings/Badge';
  import type { Profile } from '../../ipc/bindings/Profile';
  import type { NameColor } from '../../ipc/bindings/NameColor';
  import { emotesIpc } from '../../ipc/emotes';
  import { systemIpc } from '../../ipc/system';
  import { windowsIpc } from '../../ipc/windows';
  import { settings } from '../../lib/settings.svelte';
  import { streamer } from '../../lib/streamer.svelte';
  import EarnedBadges from './EarnedBadges.svelte';
  import { suspendedFlag } from './suspended';
  import { coverKey, covered } from './streamerCover';
  import { avatars } from '../emotes/avatars.svelte';
  import { looks } from '../emotes/looks.svelte';
  import { toggleWatch, watched } from './watch';
  import { copy } from '../../lib/clipboard';

  interface Props {
    profile: Profile;
    channel: string;
    /** From their last message here, when there is one. */
    badges: Badge[];
    color: NameColor | null;
    /** The card is the channel's own (no profile link needed). */
    isChannel: boolean;
    logsUrl: string;
    editing: boolean;
  }

  let { profile, channel, badges, color, isChannel, logsUrl, editing = $bindable() }: Props = $props();

  // V101-23 Per card: every card starts on the 7TV picture.
  let twitchPic = $state(false);

  const note = $derived(settings.v.chat.users[profile.id]);
  const second = $derived.by(() => {
    const n = note?.nickname || profile.displayName;
    return n.toLowerCase() === profile.login.toLowerCase() ? '' : n;
  });
  const seventv = $derived(avatars.of(profile.login));
  const picture = $derived((!twitchPic && seventv) || profile.avatar || '');

  // V108-C09 The reveal lives only here: a new opening, another user or the other picture
  // blurs it again.
  let opening = $state(0);
  let revealed = $state('');
  const shownKey = $derived(
    coverKey({ user: profile.id, source: !twitchPic && seventv ? '7tv' : 'twitch', opening }),
  );
  const blurred = $derived(covered(streamer.hides('hideAvatars'), !!picture, revealed, shownKey));
  $effect(() => {
    let stop: (() => void) | undefined;
    let gone = false;
    void windowsIpc
      .onUsercardReopened(() => {
        opening++;
        revealed = '';
      })
      .then((un) => (gone ? un() : (stop = un)))
      .catch(() => {});
    return () => {
      gone = true;
      stop?.();
    };
  });
  // V108-C16 Their 7TV account, for the profile button; none known hides it.
  let seventvId = $state<string | null>(null);
  $effect(() => {
    const id = profile.id;
    seventvId = null;
    let gone = false;
    void emotesIpc
      .seventvUser(id)
      .then((u) => !gone && (seventvId = u))
      .catch(() => {});
    return () => (gone = true);
  });
  // V109-F78 7TV only pushes a paint and badge for chatters it met in a chat open here; a card from
  // Mentions, another channel or a pop-out reads that user's profile once, and the look arrives as a push.
  $effect(() => {
    if (settings.v.chat.paints.seventv && profile.id) void emotesIpc.cardLook(profile.id).catch(() => {});
  });
  const paintId = $derived(settings.v.chat.paints.seventv ? (looks.look(profile.id)?.paint ?? null) : null);
  const paint = $derived(looks.paintClass(paintId));
  const paintName = $derived(paint ? looks.paintName(paintId) : null);
  const colorVars = $derived(color ? `--cd:${color.dark};--cl:${color.light}` : '');
  const bell = $derived(watched(settings.v.highlights.users, profile.login));
  const flag = $derived(suspendedFlag(profile.suspended));

  function flipBell() {
    void settings.patch('highlights', { users: toggleWatch(settings.v.highlights.users, profile.login) });
  }

  function copyId() {
    void copy(profile.id, 'User ID');
  }
</script>

<header class:banner={!!profile.banner} style:--banner={profile.banner ? `url("${profile.banner}")` : null}>
  <div class="pic">
    {#if blurred}
      <button class="cover" aria-label="Streamer mode is on. Show the profile picture" onclick={() => (revealed = shownKey)}>
        <span class="blur"><Avatar src={picture} name={profile.displayName} size={60} /></span>
        <span class="over">Streamer mode enabled</span>
        <span class="reveal">Click to reveal</span>
      </button>
    {:else}
      <Avatar src={picture} name={profile.displayName} size={60} />
    {/if}
    {#if seventv && profile.avatar}
      <!-- V102-12 The logo of the picture shown; a click shows the other one. -->
      <button
        class="which"
        aria-label={twitchPic ? 'Twitch picture' : '7TV picture'}
        onclick={() => {
          twitchPic = !twitchPic;
          revealed = '';
        }}
        use:tooltip={twitchPic ? 'Show the 7TV picture' : 'Show the Twitch picture'}
        ><Icon name={twitchPic ? 'twitch' : 'seventv'} size={11} /></button
      >
    {/if}
  </div>
  <div class="who">
    <!-- V107-R35 The name is text to select, not a handle for the window. -->
    <div class="name no-drag" style={colorVars}>
      <div class="names">
        <!-- V108-C17 The login leads; the display name (or your nickname) sits under it in brackets. -->
        <span class="line">
          <strong class={paint ?? ''} class:painted={!!paint} class:colored={!!color}>{profile.login}</strong>
          <button class="cp" aria-label="Copy name" use:tooltip={'Copy name'} onclick={() => copy(profile.login, 'Name')}
            ><Icon name="copy" size={11} /></button
          >
        </span>
        {#if second}
          <span class="line sub">
            <span class="display">({second})</span>
            <button class="cp" aria-label="Copy name" use:tooltip={'Copy name'} onclick={() => copy(second, 'Name')}
              ><Icon name="copy" size={11} /></button
            >
          </span>
        {/if}
      </div>
      <div class="tools">
        <IconButton
          icon="pencil"
          label="Nickname and note"
          hint="Only you see them."
          size={14}
          active={editing}
          toggle
          onclick={() => (editing = !editing)}
        />
        <IconButton
          icon="bell"
          label={bell ? 'Stop highlighting' : 'Highlight their messages'}
          hint="In every chat. Color and sound in Settings → Highlights."
          size={14}
          active={bell}
          toggle
          onclick={flipBell}
        />
        {#if logsUrl}
          <IconButton icon="scroll" label="All logs" hint="logs.cyex.app" size={14} onclick={() => systemIpc.openLink(logsUrl)} />
        {/if}
        {#if seventvId}
          <IconButton
            icon="seventv"
            label="7TV profile"
            size={14}
            onclick={() => systemIpc.openLink(`https://7tv.app/users/${seventvId}`)}
          />
        {/if}
        {#if !isChannel}
          <IconButton
            icon="twitch"
            label="Open on Twitch"
            size={14}
            onclick={() => systemIpc.openLink(`https://www.twitch.tv/${profile.login}`)}
          />
        {/if}
      </div>
    </div>
    {#if paintName}<span class="paint"><span class="fill {paint}" aria-hidden="true"></span><span class="label">{paintName}</span></span>{/if}
    <div class="marks">
      {#if flag}<span class="role suspended {flag.tone}" use:tooltip={flag.hint}>{flag.label}</span>{/if}
      {#if profile.staff}<span class="role staff">Staff</span>{/if}
      {#if profile.partner}<span class="role partner">Partner</span>{/if}
      {#if profile.affiliate}<span class="role affiliate">Affiliate</span>{/if}
    </div>
    <EarnedBadges login={profile.login} userId={profile.id} {channel} fallback={badges} />
    <!-- V110-U58 The chip copies too; the icon beside it says so. -->
    <div class="idrow">
      <button class="id" onclick={copyId} use:tooltip={'Copy user ID'}>ID {profile.id}</button>
      <button class="cp" aria-label="Copy ID" use:tooltip={'Copy ID'} onclick={copyId}><Icon name="copy" size={11} /></button>
    </div>
  </div>
</header>

<style>
  header {
    position: relative;
    display: flex;
    gap: 12px;
    padding: 12px 14px 10px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }
  /* The banner stays readable under the name in both themes. */
  header.banner {
    background:
      linear-gradient(color-mix(in srgb, var(--surface) 72%, transparent), var(--surface)),
      var(--banner) center top / cover;
  }
  /* V102-09 The picture is square (Avatar's small corner radius). */
  .pic {
    position: relative;
    flex: none;
    align-self: flex-start;
  }
  /* At the picture's corner, also with the reveal line under it. */
  .which {
    position: absolute;
    top: 47px;
    right: -6px;
    display: grid;
    place-items: center;
    width: 19px;
    height: 17px;
    padding: 0;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--muted);
  }
  .which:hover {
    color: var(--text);
  }
  /* V108-C09 The blur leaves only a smear of colour; the scrim keeps the text readable on
     any picture in both themes. */
  .cover {
    display: block;
    width: 60px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--text);
    text-align: center;
    cursor: pointer;
  }
  .blur {
    position: relative;
    display: block;
    width: 60px;
    height: 60px;
    overflow: hidden;
    border-radius: var(--radius);
  }
  .blur :global(.avatar) {
    filter: blur(12px);
    transform: scale(1.4);
  }
  .blur::after {
    content: '';
    position: absolute;
    inset: 0;
    background: color-mix(in srgb, var(--surface) 72%, transparent);
  }
  .over {
    position: absolute;
    top: 0;
    left: 0;
    display: grid;
    place-items: center;
    width: 60px;
    height: 60px;
    padding: 0 3px;
    font-size: 0.68rem;
    font-weight: 700;
    line-height: 1.15;
  }
  /* Wider than the picture: centred under it, spilling into the padding either side. */
  .reveal {
    display: flex;
    justify-content: center;
    margin-top: 3px;
    color: var(--dim);
    font-size: 0.68rem;
    white-space: nowrap;
  }
  .cover:hover .reveal {
    color: var(--text);
  }
  .who {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  /* The buttons are the end of this row, so the name gives way to their real width. */
  .name {
    display: flex;
    gap: 6px;
    align-items: flex-start;
    min-width: 0;
    font-size: 1.1rem;
  }
  .name strong {
    /* Mixed or right-to-left names keep their own order and don't pull the login around. */
    unicode-bidi: isolate;
    font-weight: 800;
    user-select: text;
    cursor: text;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .name .colored {
    color: var(--cd);
  }
  :global(:root[data-theme='light']) .name .colored {
    color: var(--cl);
  }
  .names {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }
  .line {
    display: flex;
    gap: 4px;
    align-items: center;
    min-width: 0;
  }
  .line strong {
    min-width: 0;
  }
  .display {
    min-width: 0;
    overflow: hidden;
    unicode-bidi: isolate;
    color: var(--dim);
    font-size: 0.85rem;
    text-overflow: ellipsis;
    user-select: text;
    white-space: nowrap;
  }
  .cp {
    display: grid;
    flex: none;
    place-items: center;
    width: 18px;
    height: 18px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--dim);
    cursor: pointer;
  }
  /* V110-F31 24 px to hit, 18 px to see. */
  .cp {
    position: relative;
  }
  .cp::before {
    content: '';
    position: absolute;
    inset: -3px;
  }
  .cp:hover {
    color: var(--text);
  }
  .cp:active {
    opacity: 0.6;
    transition: none;
  }
  /* The paint is the box's background (its class sets picture or gradient, and the
     shadows); a dark veil and a text shadow keep the name readable on any paint. */
  .paint {
    position: relative;
    align-self: flex-start;
    max-width: 100%;
    padding: 1px 8px;
    font-size: 0.7rem;
    font-weight: 700;
  }
  .fill {
    position: absolute;
    inset: 0;
    border-radius: var(--radius);
    background-color: var(--elevated-2);
    filter: var(--paint-shadow,);
  }
  .fill::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: rgb(0 0 0 / 0.28);
  }
  .label {
    position: relative;
    display: block;
    overflow: hidden;
    color: #fff;
    text-overflow: ellipsis;
    text-shadow: 0 1px 2px rgb(0 0 0 / 0.85);
    white-space: nowrap;
  }
  .marks {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    align-items: center;
  }
  .marks:empty {
    display: none;
  }
  .role {
    padding: 0 6px;
    border-radius: var(--radius);
    font-size: 0.7rem;
    font-weight: 700;
  }
  .staff {
    background: color-mix(in srgb, var(--red) 20%, transparent);
    color: var(--red);
  }
  .partner {
    background: color-mix(in srgb, var(--twitch) 20%, transparent);
    color: var(--twitch);
  }
  .affiliate {
    background: var(--accent-dim);
    color: var(--accent-text);
  }
  /* V102-25 */
  .suspended.red {
    background: color-mix(in srgb, var(--red) 20%, transparent);
    color: var(--red);
  }
  .suspended.yellow {
    background: color-mix(in srgb, var(--yellow) 20%, transparent);
    color: var(--yellow);
  }
  .suspended.grey {
    background: var(--surface-hover);
    color: var(--muted);
  }
  .idrow {
    display: flex;
    align-items: center;
    align-self: flex-start;
    gap: 4px;
  }
  .id {
    padding: 0;
    background: none;
    border: 0;
    color: var(--dim);
    font: 0.72rem var(--font-mono);
  }
  .id:hover {
    color: var(--text);
  }
  /* Hangs into the header's padding (6 px from the top and right edges) without making
     the row taller. */
  .tools {
    display: flex;
    flex: none;
    align-self: flex-start;
    gap: 2px;
    margin: -6px -8px -6px auto;
  }
</style>
