// V110-A11 The reader-mode switch as state, so the log and its setting row follow it.

import { readReader, writeReader } from './readerMode.ts';

class ReaderMode {
  #on = $state(readReader());
  get on(): boolean {
    return this.#on;
  }
  set(on: boolean): void {
    this.#on = on;
    writeReader(on);
  }
}

export const reader = new ReaderMode();
