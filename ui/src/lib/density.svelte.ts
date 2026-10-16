// V105-08 The screen's density as a picture scale. One value per window, so every copy of an
// emote asks for the same file; when the window moves to another screen, all switch at once.
import { pictureScale, type Scale } from './scale';

class Density {
  scale = $state<Scale>(pictureScale(devicePixelRatio));

  /** The screen's device pixel ratio; re-read when the window moves to another screen. */
  dpr(): number {
    void this.scale;
    return window.devicePixelRatio || 1;
  }

  start(): () => void {
    let query: MediaQueryList | null = null;
    const watch = () => {
      this.scale = pictureScale(devicePixelRatio);
      query?.removeEventListener('change', watch);
      query = matchMedia(`(resolution: ${devicePixelRatio}dppx)`);
      query.addEventListener('change', watch);
    };
    watch();
    return () => query?.removeEventListener('change', watch);
  }
}

export const density = new Density();
