// V110-U43 A setting someone asked to see (a link in a chat): Settings opens on its page and
// SettingsView scrolls to the row and lights it, as a search pick does. Taken once.

class SettingsJump {
  /** The setting's registry id, until Settings has shown it. */
  pending = $state<string | null>(null);

  ask(id: string): void {
    this.pending = id;
  }

  take(): string | null {
    const id = this.pending;
    this.pending = null;
    return id;
  }
}

export const settingsJump = new SettingsJump();
