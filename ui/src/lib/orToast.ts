import { errorText } from './errors';
import { toast } from './toast.svelte';

/** For a click that fires a backend call and doesn't wait for it: a refusal shows as a toast
 *  instead of dying in the console. `what` ("open the link") words the vague cases. */
export function orToast(p: Promise<unknown>, what?: string): void {
  p.catch((e) => toast.error(errorText(e, what)));
}
