// V105-07 What a follow or unfollow did, for its toast.

export interface FollowToast {
  kind: 'info' | 'error';
  text: string;
}

/** `error` is what the command threw; `undefined` when it worked. */
export function followToast(follow: boolean, name: string, error?: unknown): FollowToast {
  const verb = follow ? 'follow' : 'unfollow';
  if (error === undefined) return { kind: 'info', text: `${follow ? 'Followed' : 'Unfollowed'} ${name}` };
  const reason = typeof error === 'string' ? error.trim() : error instanceof Error ? error.message.trim() : '';
  return { kind: 'error', text: reason ? `Couldn’t ${verb}: ${reason}` : `Couldn’t ${verb} ${name}.` };
}
