// V104-01 "Open the Mentions tab" from Settings: the main window opens it, or adds it.

class MentionsTabAsk {
  asks = $state(0);

  open(): void {
    this.asks++;
  }
}

export const mentionsTab = new MentionsTabAsk();
