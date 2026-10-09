// hls.js ships types for its full build only; the light build has the same API.
declare module 'hls.js/light' {
  import Hls from 'hls.js';
  export default Hls;
}
