// V110-F40 The tab bar's side with "Above chat" remembered: turning it on stores the side it
// replaces, turning it off goes back there (this viewer's storage; lost storage means Top).

import type { TabBarSide } from '../../ipc/bindings/TabBarSide';
import { settings } from '../../lib/settings.svelte';
import { sideBeforeAbove } from './barLayout';

const KEY = 'cyex.barSideBefore';

function recall(): Exclude<TabBarSide, 'aboveChat'> {
  try {
    return sideBeforeAbove(localStorage.getItem(KEY));
  } catch {
    return 'top';
  }
}

/** Sets the side. Choosing "Above chat" from another side stores that side first; choosing
 * `back` (from the Settings toggle) leaves it for the stored one. */
export function setBarSide(value: TabBarSide | 'back'): void {
  const now = settings.v.appearance.tabBarSide;
  if (value === 'aboveChat' && now !== 'aboveChat') {
    try {
      localStorage.setItem(KEY, now);
    } catch {
      // Blocked storage: it goes back to Top.
    }
  }
  void settings.patch('appearance', { tabBarSide: value === 'back' ? recall() : value });
}
