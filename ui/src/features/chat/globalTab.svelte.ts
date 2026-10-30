// V108-GM The Global mentions tab in the title bar: its feed's name and what came while it
// wasn't shown.

import { tabsIpc } from '../../ipc/tabs';
import { feedKey, GLOBAL_FEED } from './mentionsTab';

class GlobalTab {
  unread = $state(0);
  /** The view is on screen: nothing counts as new. */
  shown = $state(false);
  private started = false;

  start(): void {
    if (this.started) return;
    this.started = true;
    void tabsIpc.onFeedNews((f) => {
      if (f === feedKey(GLOBAL_FEED) && !this.shown) this.unread++;
    });
  }

  show(on: boolean): void {
    this.shown = on;
    if (on) this.unread = 0;
  }
}

export const globalTab = new GlobalTab();
