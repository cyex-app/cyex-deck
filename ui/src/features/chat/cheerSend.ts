// V107-B The chat box's send path (P-01): a line with a cheer word of this channel goes to
// Rust as a cheer, never as text. Rust asks (threshold, the same cheer again, pasted or
// recalled) or, with Bits off, says it's plain chat. Everything else is a plain send.

import type { CheerSent } from '../../ipc/bindings/CheerSent';

/** One Enter: what the box held, trimmed, and its cheer total (0: plain chat). */
export interface Line {
  raw: string;
  value: string;
  bits: number;
  /** Ctrl+Enter: the text stays for sending again. */
  keep: boolean;
}

/** A cheer waiting for "Spend N Bits?". `again`: the same one just went out. */
export interface CheerAsk extends Line {
  again: boolean;
}

export interface SendPorts {
  cheer(value: string, bits: number, confirmed: boolean, careful: boolean): Promise<CheerSent>;
  /** Plain chat; resolves once it went out (and its UI action ran). */
  send(value: string): Promise<void>;
  /** Shows "Spend N Bits?". */
  ask(a: CheerAsk): void;
  /** A "Spend N Bits?" is open. */
  asking(): boolean;
  /** Into the Up history, the reply chip goes, the sent part leaves the box unless `keep`. */
  wentOut(raw: string, value: string, keep: boolean): void;
  balance(bits: number | null): void;
  /** Bits went off here: the gem goes too. */
  bitsOff(): void;
  /** Clears the box's error. */
  ok(): void;
  fail(e: unknown): void;
}

export class ChatSend {
  #busy = false;
  #p: SendPorts;

  constructor(p: SendPorts) {
    this.#p = p;
  }

  /** A line or cheer is on its way: a second Enter doesn't send twice. */
  get busy() {
    return this.#busy;
  }

  /** Enter. `repeat`: a held key, which never cheers. `careful`: pasted or recalled, so Rust asks. */
  submit(line: Line, repeat = false, careful = false): Promise<void> {
    if (this.#busy) return Promise.resolve();
    if (line.bits > 0) {
      if (repeat || this.#p.asking()) return Promise.resolve();
      return this.#cheer({ ...line, again: false }, false, careful);
    }
    return this.#plain(line);
  }

  /** "Spend N Bits?" said yes. */
  confirm(a: CheerAsk): Promise<void> {
    return this.#cheer(a, true, false);
  }

  async #plain({ raw, value, keep }: Line) {
    this.#busy = true;
    try {
      await this.#p.send(value);
      this.#p.ok();
      this.#p.wentOut(raw, value, keep);
    } catch (e) {
      this.#p.fail(e);
    } finally {
      this.#busy = false;
    }
  }

  async #cheer(a: CheerAsk, confirmed: boolean, careful: boolean) {
    if (this.#busy) return;
    this.#busy = true;
    /** Bits are off here: it's plain chat. */
    let asText = false;
    try {
      const r = await this.#p.cheer(a.value, a.bits, confirmed, careful);
      if (r.kind === 'ask') {
        this.#p.ok();
        return void this.#p.ask({ ...a, again: r.again });
      }
      asText = r.kind === 'text';
      if (asText) this.#p.bitsOff();
      if (r.kind === 'done') {
        this.#p.balance(r.balance);
        this.#p.ok();
        this.#p.wentOut(a.raw, a.value, a.keep);
      }
    } catch (e) {
      this.#p.fail(e);
    } finally {
      this.#busy = false;
    }
    if (asText) await this.#plain(a);
  }
}

/** What stays in the box after `raw` went out while more was typed. */
export function leftOver(box: string, raw: string, value: string): string {
  if (box.trim() === value) return '';
  if (box.startsWith(raw)) return box.slice(raw.length).trimStart();
  return box;
}
