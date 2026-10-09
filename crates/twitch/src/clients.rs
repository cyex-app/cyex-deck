//! Twitch's own public client IDs. They ship inside every twitch.tv page and Twitch app,
//! so they aren't secrets. This app's own client ID is never here: it's read from `.env`
//! at runtime (D5).

/// Android TV app. Signs in with the Device Code flow; its tokens work on GQL (D6).
pub const ANDROID_TV: &str = "ue6666qo983tsx6so1t0vnawi233wa";
/// twitch.tv website.
pub const WEB: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";
const ANDROID: &str = "kd1unb4b3q4t58fwlpcbzcbnm76a8fp";
const IOS: &str = "851cqzxpb9bqu9z6galo155du";

/// Tokens from these clients are accepted by GQL.
pub fn is_first_party(client_id: &str) -> bool {
    [ANDROID_TV, WEB, ANDROID, IOS].contains(&client_id)
}
