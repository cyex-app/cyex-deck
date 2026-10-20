// C6 What the message box suggests from: recent chatters, emotes you can use here
// (1.40) and ones seen in chat, emoji (E1), and chat bot commands (C13). Loaded once the
// box is focused.

import type { BotCommand } from '../../ipc/bindings/BotCommand';
import { emotesIpc } from '../../ipc/emotes';
import { settings } from '../../lib/settings.svelte';
import { emojiGroups, findEmoji, type EmojiGroup } from '../emotes/emoji';
import { emoteKey } from '../emotes/emotes';
import { usable } from '../emotes/menu';
import { usageRank } from '../emotes/ranking';
import { emoteMenu } from '../emotes/menus';
import { complete, type Word } from './completion';
import type { ChatRoom } from './room.svelte';
import { commandItems, emoteItems, type EmoteName, type Item, viewerLevel } from './suggestions';

export class Sources {
  private known: EmoteName[] = [];
  private emoji: EmojiGroup[] | null = null;
  private commands: BotCommand[] | null = null;
  private loading = false;

  constructor(private readonly room: ChatRoom) {}

  load() {
    if (this.loading) return;
    this.loading = true;
    const { login, account } = this.room;
    emoteMenu(login, account)
      .then((m) => {
        this.known = [...usable(m).values()].map((e) => ({ name: e.name, provider: e.provider, id: e.id, animated: e.animated }));
      })
      .catch(() => (this.loading = false));
    void emojiGroups().then((g) => (this.emoji = g));
    emotesIpc
      .botCommands(login)
      .then((c) => (this.commands = c))
      .catch(() => (this.commands = []));
  }

  /** Recent chatters, newest first. */
  chatters(): string[] {
    const names: string[] = [];
    const seen = new Set<string>();
    const messages = this.room.messages;
    for (let i = messages.length - 1; i >= 0 && names.length < 500; i--) {
      const s = messages[i].sender;
      if (s && !seen.has(s.id)) {
        seen.add(s.id);
        names.push(s.display);
      }
    }
    return names;
  }

  /** Usable emotes plus ones seen in this chat. */
  emotes(): EmoteName[] {
    const out = [...this.known];
    for (const m of this.room.messages) {
      for (const f of m.body) {
        if (f.t === 'emote') out.push({ name: f.name, provider: f.provider, id: f.id, animated: f.animated });
      }
    }
    return out;
  }

  /** V101-16 Among equal matches, emotes you use most come first: their place in the
   * most-used list (C12). */
  used(): (name: string) => number {
    const order = usageRank(settings.v.emotes.usage);
    const byName = new Map<string, number>();
    for (const e of this.emotes()) {
      const i = order.get(emoteKey(e.provider, e.id));
      if (i !== undefined && i < (byName.get(e.name) ?? Infinity)) byName.set(e.name, i);
    }
    return (name) => byName.get(name) ?? Infinity;
  }

  /** Suggestions for the word at the caret. */
  items(word: Word): Item[] {
    if (word.text.startsWith('@') && word.text.length >= 2) {
      return complete(word.text.slice(1), this.chatters()).map((n) => ({ value: `@${n}`, label: `@${n}` }));
    }
    if (word.text.startsWith(':') && word.text.length >= 3) {
      const q = word.text.slice(1).replace(/:$/, '');
      const found = this.emoji ? findEmoji(this.emoji, q, 8).map((e) => ({ char: e.char, name: e.names[0] })) : [];
      return emoteItems(q, this.emotes(), found, 8, this.used());
    }
    if (word.start === 0 && word.text.startsWith('!') && word.text.length >= 2 && this.commands) {
      const own = this.room.state.own;
      const sub = own?.badges.some((b) => b.set === 'subscriber' || b.set === 'founder') ?? false;
      return commandItems(word.text, this.commands, viewerLevel(own ? { ...own, sub } : null));
    }
    return [];
  }
}
