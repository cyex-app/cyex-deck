//! Playback (1.1) and the ad filter (1.2). The app serves the playlists built here to
//! hls.js over `cyexhls://`; segments load straight from Twitch's CDN.

pub mod ads;
pub mod backup;
pub mod fetch;
pub mod hls;
pub mod live;
pub mod stitch;
pub mod vod;
