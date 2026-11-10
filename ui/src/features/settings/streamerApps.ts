/** V110-F101 A streaming app is a program file name, so the characters Windows keeps out of
 * file names (and control characters) can't be in it. */
export function validAppName(name: string): boolean {
  const t = name.trim();
  return t !== '' && !/[<>:"/\|?*\u0000-\u001f]/.test(t);
}

/** Shown under the chips when an added name isn't one. */
export const APP_NAME_PROBLEM = 'That isn’t a program name.';
