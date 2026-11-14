//! V111-17 Which mod feed each (account, channel) gets, without tauri so it is tested.
//! Every account that is mod, lead mod or broadcaster in an open chat gets one, the default
//! account or not, the mod view open or not:
//! - EventSub (Helix sign-in) for everything; it carries mod actions and AutoMod only with the
//!   feed scopes (`channel.moderate` v2 and `moderator:manage:automod`) and when Twitch didn't
//!   refuse them for this token.
//! - Hermes (GQL sign-in) for mod actions and AutoMod where EventSub can't carry them.
//! - Neither: the chat says once that a fresh sign-in is needed (a prompt), never silently.

use std::collections::{HashMap, HashSet};

use cyex_auth::caps::Feature;

/// What an account's sign-in holds, as the planner reads it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tokens {
    /// A usable Helix token: its granted scopes and a key that changes with the token.
    pub helix: Option<(Vec<String>, u64)>,
    /// A usable GQL token.
    pub gql: bool,
}

/// The scopes EventSub's `channel.moderate` v2 and `automod.message.hold/update` v2 need.
pub fn feed_scopes() -> &'static [&'static str] {
    Feature::ModView.scopes()
}

/// (account, channel id): Twitch refused this token's mod action or AutoMod subscription.
pub type Refused = HashMap<(String, String), u64>;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Plan {
    /// Account → (channel id → login), on EventSub.
    pub eventsub: HashMap<String, HashMap<String, String>>,
    /// (account, channel id) → login, on Hermes.
    pub hermes: HashMap<(String, String), String>,
    /// (account, login): this account mods the chat but can't get its AutoMod messages.
    pub prompts: HashSet<(String, String)>,
}

/// Plans `pairs` (account, channel id, login): see the module docs.
pub fn plan(pairs: &[(String, String, String)], tokens: impl Fn(&str) -> Tokens, refused: &Refused) -> Plan {
    let mut out = Plan::default();
    for (acc, cid, login) in pairs {
        let t = tokens(acc);
        let carries = t.helix.as_ref().is_some_and(|(scopes, key)| {
            let full = feed_scopes().iter().all(|s| scopes.iter().any(|g| g == s));
            full && refused.get(&(acc.clone(), cid.clone())) != Some(key)
        });
        if t.helix.is_some() {
            out.eventsub.entry(acc.clone()).or_default().insert(cid.clone(), login.clone());
        }
        if carries {
            continue;
        }
        if t.gql {
            out.hermes.insert((acc.clone(), cid.clone()), login.clone());
        } else {
            out.prompts.insert((acc.clone(), login.clone()));
        }
    }
    out
}

/// A token's key for [`Refused`]: a new sign-in is tried again. Never shown or logged.
pub fn key_of(token: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    token.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> Vec<String> {
        cyex_auth::scopes::HELIX.iter().map(|s| (*s).to_owned()).collect()
    }

    fn pair(acc: &str, cid: &str, login: &str) -> (String, String, String) {
        (acc.into(), cid.into(), login.into())
    }

    fn tokens(acc: &str) -> Tokens {
        match acc {
            // Signed in with Twitch, every scope.
            "helix" => Tokens { helix: Some((all(), 1)), gql: false },
            // Both sign-ins.
            "both" => Tokens { helix: Some((all(), 2)), gql: true },
            // TV sign-in only (the miner's).
            "gql" => Tokens { helix: None, gql: true },
            // An old or pasted Helix token without `moderator:manage:automod`.
            "old" => Tokens {
                helix: Some((all().into_iter().filter(|s| s != "moderator:manage:automod").collect(), 3)),
                gql: false,
            },
            "old+gql" => Tokens { helix: Some((vec!["chat:read".into()], 4)), gql: true },
            _ => Tokens::default(),
        }
    }

    #[test]
    fn v111_17_every_mod_account_gets_a_feed() {
        let pairs = [pair("helix", "1", "a"), pair("both", "1", "a"), pair("gql", "2", "b")];
        let p = plan(&pairs, tokens, &Refused::new());
        assert_eq!(p.eventsub["helix"]["1"], "a");
        assert_eq!(p.eventsub["both"]["1"], "a", "not only the default account");
        assert!(!p.hermes.contains_key(&("both".into(), "1".into())), "EventSub carries it: no second copy");
        assert_eq!(p.hermes[&("gql".to_owned(), "2".to_owned())], "b");
        assert!(p.prompts.is_empty());
    }

    #[test]
    fn v111_17_missing_scope_falls_back_or_prompts() {
        let pairs = [pair("old", "1", "a"), pair("old+gql", "1", "a"), pair("none", "1", "a")];
        let p = plan(&pairs, tokens, &Refused::new());
        // Follows and the rest still come over EventSub.
        assert!(p.eventsub["old"].contains_key("1") && p.eventsub["old+gql"].contains_key("1"));
        assert!(p.hermes.contains_key(&("old+gql".into(), "1".into())), "Hermes carries AutoMod");
        assert!(p.prompts.contains(&("old".into(), "a".into())));
        assert!(p.prompts.contains(&("none".into(), "a".into())), "no usable sign-in at all");
        assert!(!p.prompts.contains(&("old+gql".into(), "a".into())));
    }

    #[test]
    fn v111_17_a_refusal_counts_for_that_token_only() {
        let pairs = [pair("helix", "1", "a"), pair("both", "1", "a"), pair("helix", "2", "b")];
        let refused = Refused::from([(("helix".into(), "1".into()), 1), (("both".into(), "1".into()), 2)]);
        let p = plan(&pairs, tokens, &refused);
        assert!(p.prompts.contains(&("helix".into(), "a".into())));
        assert!(!p.prompts.contains(&("helix".into(), "b".into())), "another channel is fine");
        assert!(p.hermes.contains_key(&("both".into(), "1".into())));
        // Signed in again: a new token, tried again.
        let stale = Refused::from([(("helix".into(), "1".into()), 99)]);
        assert!(plan(&pairs, tokens, &stale).prompts.is_empty());
    }

    #[test]
    fn v111_17_viewers_get_no_feed() {
        let p = plan(&[], tokens, &Refused::new());
        assert_eq!(p, Plan::default());
    }

    #[test]
    fn v111_17_feed_scopes_are_asked_for() {
        assert!(feed_scopes().contains(&"moderator:manage:automod"));
        assert!(feed_scopes().contains(&"moderator:read:moderators"));
        for s in feed_scopes() {
            assert!(cyex_auth::scopes::HELIX.contains(s), "{s}");
        }
    }
}
