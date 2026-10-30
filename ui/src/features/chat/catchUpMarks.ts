// V108-N18 When a chat view was closed (tab left), per read key, so coming back knows how
// long it was away. Capped: closed tiles never come back.

import { CappedMap } from '../../lib/capped';

export const awayMarks = new CappedMap<string, number>(300);
