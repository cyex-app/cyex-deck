//! Client IDs, in one place (V102-19). Client IDs are public: Twitch's ship inside every
//! twitch.tv page and Twitch app, and this app's own goes to Twitch on every sign-in. No
//! secrets here; there aren't any (the sign-in client is Public).

/// V103-01 This app's own Twitch client: "Sign in with Twitch" (Helix, IRC, EventSub).
/// Registered as a Public client, so the Device Code flow needs no secret and no
/// redirect URL. Replaces the client ID that was read from `.env` (D5).
pub const SIGN_IN: &str = "qxkwgjhfx94qiqjycyzttz1lnuzjn8";
/// The client ID for GQL, always: sign-in with a code makes tokens for it, and a pasted
/// GQL token has to be made for it (V102-19). It's Twitch's TV app (D6).
pub const GQL: &str = "ue6666qo983tsx6so1t0vnawi233wa";
/// twitch.tv website, for public reads signed out.
pub const WEB: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";
const ANDROID: &str = "kd1unb4b3q4t58fwlpcbzcbnm76a8fp";
const IOS: &str = "851cqzxpb9bqu9z6galo155du";

/// Tokens from these clients are accepted by GQL.
pub fn is_first_party(client_id: &str) -> bool {
    [GQL, WEB, ANDROID, IOS].contains(&client_id)
}
