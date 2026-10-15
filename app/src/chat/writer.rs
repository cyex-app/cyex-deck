//! V105-06 The writer connection's own status. Sends need it up; when only the writer fails
//! its login, the rooms (on the reader) stay joined, so the send error has to say why.

use std::sync::atomic::{AtomicU8, Ordering};

use cyex_chat::irc::{Client, LinkStatus};

const DOWN: u8 = 0;
const UP: u8 = 1;
const REFUSED: u8 = 2;

pub struct Writer {
    pub client: Client,
    link: AtomicU8,
}

impl Writer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            link: AtomicU8::new(DOWN),
        }
    }

    /// The client's status as it comes in. Drops reconnect on their own with the client's
    /// backoff; a refused login waits for [`Writer::retry`].
    pub fn set(&self, link: LinkStatus) {
        let v = match link {
            LinkStatus::Connected => UP,
            LinkStatus::AuthFailed => REFUSED,
            LinkStatus::Connecting | LinkStatus::Reconnecting => DOWN,
        };
        self.link.store(v, Ordering::Relaxed);
    }

    pub fn is_refused(&self) -> bool {
        self.link.load(Ordering::Relaxed) == REFUSED
    }

    /// Why a send can't go out right now, or `None` when it can.
    pub fn block(&self) -> Option<&'static str> {
        match self.link.load(Ordering::Relaxed) {
            UP => None,
            REFUSED => Some("Can’t send right now. Sign in again."),
            _ => Some("Not connected yet."),
        }
    }

    /// Tries the login again, e.g. after a new sign-in. Only a refused writer needs it.
    pub fn retry(&self) {
        if self.is_refused() {
            self.client.reconnect();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyex_chat::irc::{Anonymous, Connect};
    use std::sync::Arc;

    fn writer() -> Writer {
        // The connector never succeeds: no real Twitch connection in tests.
        let connect: Connect = Arc::new(|| Box::pin(async { Err(std::io::Error::other("test")) }));
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let (client, _task) = Client::new(Arc::new(Anonymous), connect, tx);
        Writer::new(client)
    }

    #[test]
    fn login_ok_sends() {
        let w = writer();
        assert_eq!(w.block(), Some("Not connected yet."));
        w.set(LinkStatus::Connecting);
        assert_eq!(w.block(), Some("Not connected yet."));
        w.set(LinkStatus::Connected);
        assert_eq!(w.block(), None);
    }

    #[test]
    fn refused_login_says_sign_in() {
        let w = writer();
        w.set(LinkStatus::Connecting);
        w.set(LinkStatus::AuthFailed);
        assert!(w.is_refused());
        assert_eq!(w.block(), Some("Can’t send right now. Sign in again."));
    }

    #[test]
    fn drop_then_back_needs_no_user_action() {
        let w = writer();
        w.set(LinkStatus::Connected);
        w.set(LinkStatus::Reconnecting);
        assert_eq!(w.block(), Some("Not connected yet."));
        w.set(LinkStatus::Connecting);
        w.set(LinkStatus::Connected);
        assert_eq!(w.block(), None);
    }

    #[test]
    fn refusal_clears_after_a_new_login() {
        let w = writer();
        w.set(LinkStatus::AuthFailed);
        w.set(LinkStatus::Connecting);
        assert!(!w.is_refused());
        w.set(LinkStatus::Connected);
        assert_eq!(w.block(), None);
    }
}
