//! S5 What a running plugin may do: the permissions in its info file that the user
//! agreed to, web addresses it may reach (S6 sockets too), file names in its data folder.

use std::net::IpAddr;

use url::{Host, Url};

use crate::manifest::{Manifest, Permission};

/// The permissions a plugin runs with: declared in its info file and agreed to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Granted {
    pub read_chat: bool,
    pub change_chat: bool,
    pub send: bool,
    /// `None`: no network. `Some(empty)`: any public website.
    pub network: Option<Vec<String>>,
    pub fs_read: bool,
    pub fs_write: bool,
}

impl Granted {
    /// `None` when the info file asks for something the user hasn't agreed to.
    pub fn new(m: &Manifest, agreed: &[String]) -> Option<Self> {
        if !m.keys().iter().all(|k| agreed.contains(k)) {
            return None;
        }
        let mut g = Granted::default();
        for p in &m.permissions {
            match p {
                Permission::ReadChat => g.read_chat = true,
                Permission::ChangeChat => {
                    g.read_chat = true;
                    g.change_chat = true;
                }
                Permission::SendMessages => g.send = true,
                Permission::Network { hosts } => {
                    // Any host (an empty list) wins over a list.
                    if hosts.is_empty() || g.network.as_ref().is_some_and(Vec::is_empty) {
                        g.network = Some(Vec::new());
                    } else {
                        let list = g.network.get_or_insert_with(Vec::new);
                        list.extend(hosts.iter().map(|h| h.to_ascii_lowercase()));
                    }
                }
                Permission::FilesystemRead => g.fs_read = true,
                Permission::FilesystemWrite => g.fs_write = true,
            }
        }
        Some(g)
    }

    /// Everything, for tests.
    #[cfg(test)]
    pub fn all() -> Self {
        Granted {
            read_chat: true,
            change_chat: true,
            send: true,
            network: Some(Vec::new()),
            fs_read: true,
            fs_write: true,
        }
    }
}

/// `example.com` or `*.example.com`.
pub fn valid_host_pattern(p: &str) -> bool {
    let name = p.strip_prefix("*.").unwrap_or(p);
    name.contains('.')
        && name.len() <= 253
        && name.split('.').all(|l| {
            !l.is_empty() && l.len() <= 63 && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}

/// Whether a plugin with these hosts (empty: any) may fetch `url`. Only http and https,
/// and never this computer or the local network.
pub fn check_url(url: &str, hosts: &[String]) -> Result<Url, String> {
    let u = Url::parse(url).map_err(|_| format!("Can't read the address {url}."))?;
    if !matches!(u.scheme(), "http" | "https") {
        return Err("Only http and https addresses work.".into());
    }
    let name = match u.host() {
        Some(Host::Domain(d)) => d.to_ascii_lowercase(),
        Some(Host::Ipv4(ip)) => {
            return if public(IpAddr::V4(ip)) && hosts.is_empty() {
                Ok(u)
            } else {
                Err(format!("{ip} isn't allowed."))
            };
        }
        Some(Host::Ipv6(ip)) => {
            return if public(IpAddr::V6(ip)) && hosts.is_empty() {
                Ok(u)
            } else {
                Err(format!("{ip} isn't allowed."))
            };
        }
        None => return Err("The address has no host.".into()),
    };
    if name == "localhost" || name.ends_with(".localhost") || !name.contains('.') {
        return Err(format!("{name} isn't allowed."));
    }
    if hosts.is_empty() || hosts.iter().any(|p| host_matches(p, &name)) {
        Ok(u)
    } else {
        Err(format!("{name} isn't in this plugin's list of websites."))
    }
}

fn host_matches(pattern: &str, host: &str) -> bool {
    match pattern.strip_prefix("*.") {
        Some(base) => host == base || host.strip_suffix(base).is_some_and(|rest| rest.ends_with('.')),
        None => host == pattern,
    }
}

/// S6 A socket address (`ws://` or `wss://`) under the same rules as web addresses.
pub fn check_ws_url(url: &str, hosts: &[String]) -> Result<Url, String> {
    let u = Url::parse(url).map_err(|_| format!("Can't read the address {url}."))?;
    let web = match u.scheme() {
        "ws" => "http",
        "wss" => "https",
        _ => return Err("Only ws and wss addresses work.".into()),
    };
    let mut probe = u.clone();
    probe.set_scheme(web).map_err(|()| format!("Can't read the address {url}."))?;
    check_url(probe.as_str(), hosts)?;
    Ok(u)
}

/// Not a local, private or reserved address.
pub(crate) fn public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let [a, b, ..] = v4.octets();
            !(v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_multicast()
                || v4.is_documentation()
                || a == 0
                || (a == 100 && (64..128).contains(&b)))
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return public(IpAddr::V4(v4));
            }
            let first = v6.segments()[0];
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (first & 0xfe00) == 0xfc00
                || (first & 0xffc0) == 0xfe80)
        }
    }
}

/// A file in the data folder: letters, digits, `.`, `_`, `-`, not starting with a dot.
pub fn data_name(name: &str) -> Result<&str, String> {
    let ok = !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if ok {
        Ok(name)
    } else {
        Err(format!("\"{name}\" can't be a file name here."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(perms: &str) -> Manifest {
        Manifest::parse(&format!(
            r#"{{"name": "x", "version": "1", "permissions": {perms}}}"#
        ))
        .unwrap()
    }

    #[test]
    fn runs_only_with_what_was_agreed() {
        let m = manifest(r#"[{"type": "ChangeChat"}, {"type": "Network", "hosts": ["a.com"]}]"#);
        assert_eq!(Granted::new(&m, &["ChangeChat".into()]), None);
        let g = Granted::new(
            &m,
            &["ChangeChat".into(), "Network:a.com".into(), "SendMessages".into()],
        )
        .unwrap();
        assert!(g.read_chat && g.change_chat && !g.send && !g.fs_read);
        assert_eq!(g.network, Some(vec!["a.com".into()]));
        // An update that asks for more waits for a new OK.
        let more = manifest(r#"[{"type": "ChangeChat"}, {"type": "Network", "hosts": ["a.com", "b.com"]}]"#);
        assert_eq!(
            Granted::new(&more, &["ChangeChat".into(), "Network:a.com".into()]),
            None
        );
    }

    #[test]
    fn any_host_wins_over_a_list() {
        let m = manifest(r#"[{"type": "Network", "hosts": ["a.com"]}, {"type": "Network"}]"#);
        let g = Granted::new(&m, &m.keys()).unwrap();
        assert_eq!(g.network, Some(vec![]));
    }

    #[test]
    fn sockets_follow_the_same_rules() {
        let list = vec!["feed.example.com".to_owned()];
        assert_eq!(
            check_ws_url("wss://feed.example.com/live", &list).map(|u| u.scheme().to_owned()),
            Ok("wss".into())
        );
        assert!(check_ws_url("ws://feed.example.com", &list).is_ok());
        assert!(check_ws_url("https://feed.example.com", &list).is_err());
        assert!(check_ws_url("wss://other.example.com", &list).is_err());
        assert!(check_ws_url("ws://localhost:8080", &[]).is_err());
        assert!(check_ws_url("ws://192.168.1.1", &[]).is_err());
    }

    #[test]
    fn urls_stay_on_the_listed_public_hosts() {
        let list = vec!["api.example.com".to_owned(), "*.example.org".to_owned()];
        assert!(check_url("https://api.example.com/x?y=1", &list).is_ok());
        assert!(check_url("https://cdn.example.org/a", &list).is_ok());
        assert!(check_url("https://example.org/a", &list).is_ok());
        assert!(check_url("https://evil-example.org/a", &list).is_err());
        assert!(check_url("https://example.com/", &list).is_err());
        assert!(check_url("https://anything.net/", &[]).is_ok());
        for bad in [
            "file:///C:/Windows/win.ini",
            "ftp://example.com/",
            "http://localhost:8080/",
            "http://app.localhost/",
            "http://127.0.0.1/",
            "http://192.168.1.1/",
            "http://10.0.0.5/",
            "http://169.254.169.254/latest",
            "http://[::1]/",
            "http://[::ffff:127.0.0.1]/",
            "http://[fd00::1]/",
            "http://intranet/",
        ] {
            assert!(check_url(bad, &[]).is_err(), "{bad}");
        }
        assert!(check_url("http://1.1.1.1/", &[]).is_ok());
        assert!(check_url("http://1.1.1.1/", &list).is_err());
    }

    #[test]
    fn data_names_stay_in_the_folder() {
        assert!(data_name("notes.txt").is_ok());
        for bad in [
            "",
            "../x",
            "a/b",
            "a\\b",
            ".hidden",
            "C:x",
            "con:",
            &"a".repeat(65),
        ] {
            assert!(data_name(bad).is_err(), "{bad}");
        }
    }
}
