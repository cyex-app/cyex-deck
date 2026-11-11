// V110-N5 R2-3 The toast after Undo: the automations note only when they were on before and are off now.
export function putBackText(wasOn: boolean, onNow: boolean): string {
  return wasOn && !onNow ? 'Put back. Automations are off until you switch them on.' : 'Put back.';
}
