// V110-F75 The one gate in front of every link-preview picture: only the Chatterino
// resolver's picture proxy is ever loaded, so no link host or image host sees this machine.
// Rust already rewrites pictures to the proxy; this stops a later code path from slipping
// a direct URL through. Anything else shows no picture.

const PROXY_HOST = 'braize.pajlada.com';
const PROXY_PATH = '/chatterino/';

/** The URL when it is a picture from the resolver's proxy, else `undefined`. */
export function previewPic(url: string | null | undefined): string | undefined {
  if (!url) return undefined;
  let u: URL;
  try {
    u = new URL(url);
  } catch {
    return undefined;
  }
  const ok = u.protocol === 'https:' && u.hostname === PROXY_HOST && !u.port && !u.username && !u.password && u.pathname.startsWith(PROXY_PATH);
  return ok ? url : undefined;
}
