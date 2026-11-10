// C6 What the message box suggests from: recent chatters, emotes you can use here
// (1.40) and ones seen in chat, emoji (E1), and chat bot commands (C13). Loaded once the
// box is focused.

import type { EmoteMenu } from '../../ipc/bindings/EmoteMenu';
import type { BotCommand } from '../../ipc/bindings/BotCommand';
import { emotesIpc } from '../../ipc/emotes';
import { emojiGroups, findEmoji, type EmojiGroup } from '../emotes/emoji';
import { completionEmotes } from '../emotes/owned';
import { providerRank } from '../emotes/priority';
import { emoteMenu } from '../emotes/menus';
import type { Word } from './completion';
import { matchPeople, mentionName, type Person } from './people';
import type { ChatRoom } from './room.svelte';
import { commandItems, emoteList, type EmoteName, type Item, viewerLevel } from './suggestions';

export class Sources {
  private menu: EmoteMenu | null = null;
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
        this.menu = m;
      })
      .catch(() => (this.loading = false));
    void emojiGroups()
      .then((g) => (this.emoji = g))
      .catch(() => {});
    emotesIpc
      .botCommands(login)
      .then((c) => (this.commands = c))
      .catch(() => (this.commands = []));
  }

  /** Recent chatters, newest first. V110-F86 `name` is what a mention says: the login for a localized display name. */
  people(): Person[] {
    const out: Person[] = [];
    const seen = new Set<string>();
    const messages = this.room.messages;
    for (let i = messages.length - 1; i >= 0 && out.length < 500; i--) {
      const s = messages[i].sender;
      if (s && !seen.has(s.id)) {
        seen.add(s.id);
        out.push({ name: mentionName(s.display, s.login), display: s.display });
      }
    }
    return out;
  }

  /** The names a mention can say, newest first. */
  chatters(): string[] {
    return this.people().map((p) => p.name);
  }

  /** Usable emotes plus ones seen in this chat (V107-F10: seen Twitch emotes only when yours). */
  emotes(): EmoteName[] {
    const seen: EmoteName[] = [];
    for (const m of this.room.messages) {
      for (const f of m.body) {
        if (f.t === 'emote') seen.push({ name: f.name, provider: f.provider, id: f.id, animated: f.animated });
      }
    }
    return completionEmotes(this.menu, seen);
  }

  /** Suggestions for the word at the caret. */
  items(word: Word): Item[] {
    return this.list(word).items;
  }

  /** V110-A-idea The suggestions and how many more matched than they show. */
  list(word: Word): { items: Item[]; more: number } {
    if (word.text.startsWith('@') && word.text.length >= 2) {
      return { items: matchPeople(word.text.slice(1), this.people()).map((n) => ({ value: `@${n}`, label: `@${n}` })), more: 0 };
    }
    if (word.text.startsWith(':') && word.text.length >= 3) {
      const q = word.text.slice(1).replace(/:$/, '');
      const found = this.emoji ? findEmoji(this.emoji, q, 8).map((e) => ({ char: e.char, name: e.names[0] })) : [];
      return emoteList(q, this.emotes(), found, 8, providerRank());
    }
    if (word.start === 0 && word.text.startsWith('!') && word.text.length >= 2 && this.commands) {
      const own = this.room.state.own;
      const sub = own?.badges.some((b) => b.set === 'subscriber' || b.set === 'founder') ?? false;
      return { items: commandItems(word.text, this.commands, viewerLevel(own ? { ...own, sub } : null)), more: 0 };
    }
    return { items: [], more: 0 };
  }
}
