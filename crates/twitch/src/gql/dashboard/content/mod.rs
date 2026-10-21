//! V107-R39 Your channel's clips and videos, the way Twitch's dashboard manages them
//! (Content › Clips, Video Producer). Operation texts come from dashboard.twitch.tv's web
//! JS (2026/10/21, docs/research/twitch-api.md); lists were checked live as cyexbot, the
//! writes only against a local mock. Every call takes the dashboard's `Channel` (V107-R41).

pub mod clips;
pub mod videos;
