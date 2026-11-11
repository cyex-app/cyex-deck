//! V110-F87 Where `$request` may go: the public web only. A command (your own, an imported
//! one, an automation set off by a chatter) can put the answer in chat, so this PC, the
//! router, the home network and cloud metadata addresses are out, also behind a redirect or
//! a name that resolves there. (`cyex_plugins::perm` has the same list for plugins; it isn't
//! public, so the two overlap for now.)

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};

use url::{Host, Url};

/// Redirects followed, each checked like the first address.
pub const MAX_HOPS: usize = 3;

pub const REFUSED: &str = "That address isn’t on the public web.";

/// Not a local, private or reserved address.
pub fn public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let [a, b, c, _] = v4.octets();
            !(v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_multicast()
                || v4.is_documentation()
                || a == 0
                // Carrier NAT, protocol assignments, benchmarking, reserved.
                || (a == 100 && (64..128).contains(&b))
                || (a == 192 && b == 0 && c == 0)
                || (a == 198 && (b & 0xfe) == 18)
                || a >= 240)
        }
        IpAddr::V6(v6) => {
            // An IPv4 address carried inside is judged as that address.
            if let Some(v4) = embedded_v4(&v6) {
                return public(IpAddr::V4(v4));
            }
            let s = v6.segments();
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                // Unique local, link-local, the old site-local.
                || (s[0] & 0xfe00) == 0xfc00
                || (s[0] & 0xffc0) == 0xfe80
                || (s[0] & 0xffc0) == 0xfec0
                // Teredo (an IPv4 address in disguise), documentation, local NAT64.
                || (s[0] == 0x2001 && s[1] == 0)
                || (s[0] == 0x2001 && s[1] == 0x0db8)
                || (s[0] == 0x64 && s[1] == 0xff9b && s[2] == 1))
        }
    }
}

/// `::ffff:a.b.c.d`, `::a.b.c.d`, NAT64 `64:ff9b::a.b.c.d` and 6to4 `2002:aabb:ccdd::`.
fn embedded_v4(v6: &Ipv6Addr) -> Option<Ipv4Addr> {
    let s = v6.segments();
    let from = |hi: u16, lo: u16| {
        let ([a, b], [c, d]) = (hi.to_be_bytes(), lo.to_be_bytes());
        Ipv4Addr::new(a, b, c, d)
    };
    match s {
        [0, 0, 0, 0, 0, 0xffff, hi, lo] => Some(from(hi, lo)),
        // `::` and `::1` are IPv6's own.
        [0, 0, 0, 0, 0, 0, hi, lo] if (hi, lo) != (0, 0) && (hi, lo) != (0, 1) => Some(from(hi, lo)),
        [0x64, 0xff9b, 0, 0, 0, 0, hi, lo] => Some(from(hi, lo)),
        [0x2002, hi, lo, ..] => Some(from(hi, lo)),
        _ => None,
    }
}

/// Names that only mean something on this PC or network, whatever they resolve to.
fn local_name(name: &str) -> bool {
    let name = name.trim_end_matches('.').to_ascii_lowercase();
    const ENDS: [&str; 6] = ["localhost", "local", "internal", "lan", "home.arpa", "intranet"];
    // One word ("router", "nas") is a name of this network.
    !name.contains('.') || ENDS.iter().any(|e| name == *e || name.ends_with(&format!(".{e}")))
}

/// An http(s) address that isn't local by its looks: the scheme, the name, a literal IP.
/// What the name resolves to is [`resolve`]'s part.
pub fn looks_public(url: &Url) -> bool {
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    match url.host() {
        Some(Host::Domain(name)) => !local_name(name),
        Some(Host::Ipv4(ip)) => public(IpAddr::V4(ip)),
        Some(Host::Ipv6(ip)) => public(IpAddr::V6(ip)),
        None => false,
    }
}

/// The address to connect to, when every address the name has is public. The request is then
/// pinned to it, so a second lookup can't answer something else.
pub fn resolve(url: &Url) -> Result<SocketAddr, String> {
    if !looks_public(url) {
        return Err(REFUSED.into());
    }
    let host = url.host_str().ok_or(REFUSED)?;
    let port = url.port_or_known_default().ok_or(REFUSED)?;
    let addrs: Vec<SocketAddr> = (host.trim_matches(['[', ']']), port)
        .to_socket_addrs()
        .map_err(|_| "That address doesn’t exist.".to_owned())?
        .collect();
    pick(&addrs)
}

fn pick(addrs: &[SocketAddr]) -> Result<SocketAddr, String> {
    if addrs.iter().any(|a| !public(a.ip())) {
        return Err(REFUSED.into());
    }
    // V110-N1 An IPv4 address first: a host with a dead IPv6 route still connects.
    addrs.iter().find(|a| a.is_ipv4()).or(addrs.first()).copied().ok_or_else(|| "That address doesn’t exist.".to_owned())
}

/// Where a redirect leads, and whether your own headers go along: only to the same host,
/// port and scheme (they can hold a key meant for the first address).
pub fn next_hop(from: &Url, location: &str) -> Result<(Url, bool), String> {
    let to = from.join(location).map_err(|_| REFUSED.to_owned())?;
    if !looks_public(&to) {
        return Err(REFUSED.into());
    }
    let same = to.origin() == from.origin();
    Ok((to, same))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn v110_n1_pick_prefers_ipv4_and_still_refuses_private() {
        let a = |s: &str| s.parse::<SocketAddr>().unwrap();
        let both = [a("[2606:4700::1111]:443"), a("1.1.1.1:443")];
        assert_eq!(pick(&both).unwrap(), a("1.1.1.1:443"));
        assert_eq!(pick(&both[..1]).unwrap(), a("[2606:4700::1111]:443"));
        assert!(pick(&[a("[2606:4700::1111]:443"), a("10.0.0.1:443")]).is_err());
    }

    #[test]
    fn local_and_reserved_addresses_are_refused() {
        for ip in [
            "127.0.0.1", "127.8.9.1", "10.0.0.1", "172.16.0.1", "172.31.255.255", "192.168.1.1", "169.254.169.254",
            "0.0.0.0", "0.1.2.3", "100.64.0.1", "100.127.255.255", "192.0.0.1", "198.18.0.1", "198.19.255.255",
            "224.0.0.1", "239.255.255.250", "240.0.0.1", "255.255.255.255", "192.0.2.1", "203.0.113.5",
            "::", "::1", "fe80::1", "febf::1", "fc00::1", "fdff::1", "fec0::1", "ff02::1", "2001:db8::1",
            "::ffff:127.0.0.1", "::ffff:10.0.0.1", "::127.0.0.1", "64:ff9b::7f00:1", "64:ff9b::a9fe:a9fe",
            "64:ff9b:1::1", "2002:7f00:1::", "2002:c0a8:101::1", "2001:0:4136:e378:8000:63bf:3fff:fdd2",
        ] {
            assert!(!public(ip.parse().unwrap()), "{ip}");
        }
        for ip in [
            "1.1.1.1", "8.8.8.8", "100.63.255.255", "100.128.0.1", "172.32.0.1", "192.0.1.1", "198.20.0.1",
            "223.255.255.255", "2606:4700:4700::1111", "2a00:1450:4001::200e", "::ffff:1.1.1.1", "64:ff9b::101:101",
            "2002:101:101::1",
        ] {
            assert!(public(ip.parse().unwrap()), "{ip}");
        }
    }

    #[test]
    fn addresses_that_look_local_never_get_a_lookup() {
        for bad in [
            "http://localhost/", "http://LOCALHOST:8080/", "http://localhost./", "http://a.localhost/",
            "http://printer.local/", "http://db.internal/", "http://nas.lan/", "http://x.home.arpa/",
            "http://router/", "http://intranet/", "https://wiki.intranet/",
            "http://127.0.0.1/", "http://127.1/", "http://2130706433/", "http://0x7f.0.0.1/", "http://0177.0.0.1/",
            "http://0/", "http://[::1]/", "http://[::ffff:127.0.0.1]/", "http://[::ffff:7f00:1]/",
            "http://192.168.0.1/admin", "http://169.254.169.254/latest/meta-data/", "http://[fd00::1]/",
            "http://user:pw@127.0.0.1/", "http://127.0.0.1.:80/",
            "ftp://example.com/", "file:///C:/Windows/win.ini", "data:text/plain,hi", "javascript:alert(1)",
        ] {
            assert!(!looks_public(&url(bad)), "{bad}");
            assert_eq!(resolve(&url(bad)), Err(REFUSED.into()), "{bad}");
        }
        for ok in ["https://example.com/a.json", "http://api.example.co.uk:8080/x?y=1", "https://1.1.1.1/", "https://127.0.0.1.example.com/"] {
            assert!(looks_public(&url(ok)), "{ok}");
        }
    }

    #[test]
    fn a_name_with_any_private_address_is_refused() {
        let a = |s: &str| -> SocketAddr { format!("{s}:443").parse().unwrap() };
        assert_eq!(pick(&[a("1.1.1.1"), a("8.8.8.8")]), Ok(a("1.1.1.1")));
        // One public and one private answer: a rebinding name.
        assert_eq!(pick(&[a("1.1.1.1"), a("127.0.0.1")]), Err(REFUSED.into()));
        assert_eq!(pick(&[a("192.168.1.1")]), Err(REFUSED.into()));
        assert_eq!(pick(&[a("[::1]")]), Err(REFUSED.into()));
        assert!(pick(&[]).is_err());
    }

    #[test]
    fn redirects_are_checked_and_keys_stay_with_their_host() {
        let from = url("https://api.example.com/v1/x");
        assert_eq!(next_hop(&from, "/v2/x"), Ok((url("https://api.example.com/v2/x"), true)));
        assert_eq!(next_hop(&from, "https://cdn.example.net/x"), Ok((url("https://cdn.example.net/x"), false)));
        assert_eq!(next_hop(&from, "http://api.example.com/v1/x").map(|n| n.1), Ok(false), "another scheme");
        assert_eq!(next_hop(&from, "https://api.example.com:8443/").map(|n| n.1), Ok(false), "another port");
        for bad in [
            "http://127.0.0.1:9229/json", "//localhost/x", "http://169.254.169.254/", "http://[::1]/",
            "file:///C:/Windows/win.ini", "http://router.lan/", "http://2130706433/",
        ] {
            assert_eq!(next_hop(&from, bad), Err(REFUSED.into()), "{bad}");
        }
    }
}
