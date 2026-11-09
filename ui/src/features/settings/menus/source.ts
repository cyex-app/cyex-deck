// V110-U67 The menus the manager lists, in the registry's order, and the fake contexts it
// previews them with.

import { MENU_DEFS, type AnyDef, menuInfo } from '../../../components/menus/defs/index.ts';
import type { SampleOf } from './model.ts';

export const DEFS: AnyDef[] = MENU_DEFS;

export const sampleOf: SampleOf = (menuId, who) => menuInfo(menuId)?.sample(who);

export const defOf = (id: string): AnyDef | undefined => DEFS.find((d) => d.id === id);
