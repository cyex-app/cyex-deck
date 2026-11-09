// V110-F75 The one gate in front of every link-preview picture: only our own resolver's
// `/v1/img` (V110-U70) and Chatterino's picture proxy are ever loaded, so no link host or image
// host sees this machine. Rust already rewrites pictures to one of the two; this stops a later
// code path from slipping a direct URL through. Anything else shows no picture.

const PROXY_HOST = 'braize.pajlada.com';
const PROXY_PATH = '/chatterino/';
const OWN_HOST = 'embed.cyex.app';
const OWN_PATH = '/v1/img';

/** The URL when it is a picture from our resolver or Chatterino's proxy, else `undefined`. */
export function previewPic(url: string | null | undefined): string | undefined {
  if (!url) return undefined;
  let u: URL;
  try {
    u = new URL(url);
  } catch {
    return undefined;
  }
  if (u.protocol !== 'https:' || u.port || u.username || u.password) return undefined;
  const chatterino = u.hostname === PROXY_HOST && u.pathname.startsWith(PROXY_PATH);
  const own = u.hostname === OWN_HOST && u.pathname === OWN_PATH && u.search.length > '?u='.length && u.search.startsWith('?u=');
  return chatterino || own ? url : undefined;
}
