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

/// V110-F15 Image hosts a card's picture may come from besides the trusted domains: the
/// CDNs of the sites above and of Twitch. `*.x` is any subdomain of `x`; the rest are exact.
pub const MEDIA_HOSTS: &[&str] = &[
    "*.cdninstagram.com",
    "*.fbcdn.net",
    "*.giphy.com",
    "*.twimg.com",
    "*.ytimg.com",
    "cdn.7tv.app",
    "cdn.discordapp.com",
    "clips-media-assets2.twitch.tv",
    "external-preview.redd.it",
    "i.cyex.app",
    "i.gyazo.com",
    "i.imgur.com",
    "i.redd.it",
    "i.ytimg.com",
    "kappa.lol",
    "media.discordapp.net",
    "media.giphy.com",
    "media.tenor.com",
    "pbs.twimg.com",
    "preview.redd.it",
    "static-cdn.jtvnw.net",
];

/// `host` ends in `.suffix` with something before it, ignoring case.
fn under(host: &str, suffix: &str) -> bool {
    let (h, s) = (host.as_bytes(), suffix.as_bytes());
    h.len() > s.len() + 1 && h[h.len() - s.len() - 1] == b'.' && h[h.len() - s.len()..].eq_ignore_ascii_case(s)
}

/// `host` is a listed domain or a subdomain of one. Whole labels only: `evil-x.com` and
/// `x.com.evil.io` are not `x.com`.
pub fn trusted_host(host: &str) -> bool {
    TRUSTED_HOSTS.iter().any(|d| host.eq_ignore_ascii_case(d) || under(host, d))
}

/// `host` may serve a card's picture: a trusted host or a listed media CDN.
pub fn media_host(host: &str) -> bool {
    trusted_host(host)
        || MEDIA_HOSTS.iter().any(|d| match d.strip_prefix("*.") {
            Some(base) => under(host, base),
            None => host.eq_ignore_ascii_case(d),
        })
}

/// An https link on a trusted host, without sign-in part or port.
fn https_on(url: &str, host_ok: fn(&str) -> bool) -> bool {
    let Ok(u) = Url::parse(url) else { return false };
    u.scheme() == "https"
        && u.username().is_empty()
        && u.password().is_none()
        && u.port().is_none()
        && u.host_str().is_some_and(host_ok)
}

/// A link that may be embedded: https, no sign-in part, the default port, a trusted host.
pub fn trusted(url: &str) -> bool {
    https_on(url, trusted_host)
}

/// A picture a card may load without a hover: same checks, on trusted hosts and media CDNs.
pub fn trusted_media(url: &str) -> bool {
    https_on(url, media_host)
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

    #[test]
    fn idn_uppercase_and_dot_hosts() {
        // Look-alikes in other scripts become punycode and match nothing.
        for u in [
            "https://\u{0445}.com/a",
            "https://youtub\u{0435}.com/a",
            "https://\u{0445}.com.evil.io/",
            "https://xn--e1awd7f.com/",
            "https://x.com.\u{0440}u/",
        ] {
            assert!(!trusted(u), "{u}");
        }
        // Case never matters, in the link or in a bare host.
        assert!(trusted("https://WWW.YouTube.COM/watch?v=a"));
        assert!(trusted_host("X.COM") && trusted_host("Clips.Twitch.TV"));
        // A trailing dot is a different name here.
        assert!(!trusted("https://youtube.com./a"));
        assert!(!trusted_host("x.com."));
        assert!(!trusted_media("https://i.ytimg.com./a.jpg"));
    }

    #[test]
    fn card_pictures_from_listed_hosts_only() {
        for u in [
            "https://i.ytimg.com/vi/a/hq.jpg",
            "https://img.ytimg.com/a.jpg",
            "https://pbs.twimg.com/media/a.jpg",
            "https://abs.twimg.com/a.png",
            "https://preview.redd.it/a.jpg",
            "https://external-preview.redd.it/a.jpg",
            "https://scontent.cdninstagram.com/a.jpg",
            "https://scontent-x.xx.fbcdn.net/a.jpg",
            "https://static-cdn.jtvnw.net/a.jpg",
            "https://clips-media-assets2.twitch.tv/a.jpg",
            "https://media.tenor.com/a.gif",
            "https://i.gyazo.com/a.png",
            "https://MEDIA.GIPHY.com/a.gif",
            "https://i.imgur.com/a.png",
        ] {
            assert!(trusted_media(u), "{u}");
        }
        for u in [
            "http://i.ytimg.com/a.jpg",
            "https://ytimg.com/a.jpg",
            "https://twimg.com/a.jpg",
            "https://evil-ytimg.com/a.jpg",
            "https://i.ytimg.com.evil.io/a.jpg",
            "https://tracker.example.com/pixel.gif",
            "https://i.ytimg.com@evil.io/a.jpg",
            "https://i.ytimg.com:8443/a.jpg",
            "https://static-cdn.jtvnw.net.evil.io/a.jpg",
            "https://cdn.jtvnw.net/a.jpg",
            "",
        ] {
            assert!(!trusted_media(u), "{u}");
        }
    }
}
