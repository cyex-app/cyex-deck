/// Sign-in errors, worded for the UI.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Twitch turned the app’s sign-in down. Try again later.")]
    BadClientId,
    #[error("Couldn’t reach Twitch. Check your connection and try again.")]
    Offline,
    #[error("That token doesn’t work. It may have expired or been revoked.")]
    BadToken,
    #[error("That’s an app token. Paste one that belongs to a Twitch account.")]
    AppToken,
    #[error("That doesn’t look like a token.")]
    NotAToken,
    /// V102-19 A token or code for another account than the one it's added to.
    #[error("That’s {0}, not this account.")]
    OtherUser(String),
    /// V102-19 A pasted token made for another app than GQL.
    #[error("That token isn’t a GQL token.")]
    NotGql,
    /// V102-19 A GQL token pasted for an account that isn't here.
    #[error("Sign in with Twitch first.")]
    NoAccount,
    #[error("The code expired. Start again.")]
    Expired,
    #[error("Sign-in was declined on Twitch.")]
    Denied,
    #[error("Cancelled.")]
    Cancelled,
    #[error("Couldn’t save the token: {0}")]
    Store(String),
    #[error("Twitch said: {0}")]
    Twitch(String),
}

impl From<cyex_twitch::Error> for AuthError {
    fn from(e: cyex_twitch::Error) -> Self {
        if e.is_network() {
            AuthError::Offline
        } else if e.is_unauthorized() {
            AuthError::BadToken
        } else {
            AuthError::Twitch(e.api_message().unwrap_or("unexpected answer").to_owned())
        }
    }
}
