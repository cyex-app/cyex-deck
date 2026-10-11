/// Sign-in errors, worded for the UI.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Add your Twitch client ID first.")]
    NoClientId,
    #[error("Twitch doesn't recognize your client ID. Check it under Twitch client ID.")]
    BadClientId,
    #[error("Couldn't reach Twitch. Check your connection and try again.")]
    Offline,
    #[error("That token doesn't work. It may have expired or been revoked.")]
    BadToken,
    #[error("That's an app token. Paste one that belongs to a Twitch account.")]
    AppToken,
    #[error("That doesn't look like a token.")]
    NotAToken,
    /// V101-08 The manual screen's name or user ID isn't the token's.
    #[error("That token belongs to {0}.")]
    OtherUser(String),
    /// V101-08 The manual screen's client ID isn't the one the token was made for.
    #[error("That token was made for client ID {0}.")]
    OtherClient(String),
    #[error("The code expired. Start again.")]
    Expired,
    #[error("Sign-in was declined on Twitch.")]
    Denied,
    #[error("Cancelled.")]
    Cancelled,
    #[error("Couldn't save the token: {0}")]
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
