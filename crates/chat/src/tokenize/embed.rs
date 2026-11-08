//! V110-C66 Which links may be embedded under a message or asked for ahead of a hover. Only
//! https links on a listed domain or one of its subdomains; every other link is asked for on
//! a hover alone (V108-F46). The one list lives here, so the page can't widen it.

use url::Url;

/// Trusted domains. Each also covers its subdomains. Sorted by nothing in particular; keep
/// to sites and image hosts people post links to.
pub const TRUSTED_HOSTS: &[&str] = &[
    "7tv.app",
    "cdn.discordapp.com",
    "giphy.com",
    "gyazo.com",
    "i.cyex.app",
    "imgur.com",
    "instagram.com",
    "kappa.lol",
    "media.discordapp.net",
    "redd.it",
    "reddit.com",
    "streamable.com",
    "tenor.com",
    "twitch.tv",
    "twitter.com",
    "x.com",
    "youtu.be",
    "youtube.com",
];

/// `host` is a listed domain or a subdomain of one. Whole labels only: `evil-x.com` and
/// `x.com.evil.io` are not `x.com`.
pub fn trusted_host(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    TRUSTED_HOSTS
        .iter()
        .any(|d| host == *d || host.strip_suffix(d).is_some_and(|rest| rest.ends_with('.') && rest.len() > 1))
}

/// A link that may be embedded: https, no sign-in part, the default port, a trusted host.
pub fn trusted(url: &str) -> bool {
    let Ok(u) = Url::parse(url) else { return false };
    u.scheme() == "https"
        && u.username().is_empty()
        && u.password().is_none()
        && u.port().is_none()
        && u.host_str().is_some_and(trusted_host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listed_sites_and_subdomains() {
        for u in [
            "https://x.com/a/status/1",
            "https://www.youtube.com/watch?v=abc",
            "https://youtu.be/abc",
            "https://clips.twitch.tv/Funny",
            "https://i.imgur.com/a.png",
            "https://i.redd.it/a.jpg",
            "https://kappa.lol/abc",
            "https://i.cyex.app/x.png",
            "https://cdn.discordapp.com/attachments/1/2/a.png",
            "https://MEDIA.discordapp.net/a.png",
            "https://cdn.7tv.app/emote/1",
        ] {
            assert!(trusted(u), "{u}");
        }
    }

    #[test]
    fn look_alikes_fail() {
        for u in [
            "https://evil-youtube.com/x",
            "https://youtube.com.evil.io/x",
            "https://notx.com/x",
            "https://x.com.evil.io/",
            "https://evilx.com/",
            "https://discordapp.com/x",
            "https://cdn.discordapp.com.evil.io/x",
            "https://example.com/youtube.com",
            "https://example.com/?u=https://x.com",
            "https://x.com@evil.io/",
            "https://youtube.com:8443/x",
            "https://i.cyex.app.evil.io/",
            "https://cyex.app/x",
        ] {
            assert!(!trusted(u), "{u}");
        }
    }

    #[test]
    fn https_only_and_garbage() {
        assert!(!trusted("http://x.com/a"));
        assert!(!trusted("ftp://x.com/a"));
        assert!(!trusted("x.com/a"));
        assert!(!trusted(""));
        assert!(!trusted("https://user:pw@x.com/"));
        assert!(!trusted("https://x.com./a"));
        assert!(!trusted_host(""));
        assert!(!trusted_host(".x.com"));
    }
}
