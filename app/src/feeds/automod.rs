//! V106-07 AutoMod feed (Q2, Q3): held messages from every channel an account moderates,
//! from EventSub and Hermes alike. Decided rows stay, dimmed; pending ones are never pushed
//! out by decided ones (Q4).

use std::collections::HashSet;

use cyex_settings::schema::FeedKind;
use serde::Serialize;
use tauri::AppHandle;
use ts_rs::TS;

use super::{FeedEntry, push, remove, update};
use crate::moderation::view::HeldItem;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum HeldStatus {
    Pending,
    Allowed,
    Denied,
    Expired,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HeldEntry {
    /// The message id.
    pub key: String,
    #[ts(type = "number")]
    pub ts: i64,
    pub account: String,
    pub channel_id: String,
    /// Login, shown as #login (Q2).
    pub channel: String,
    pub item: HeldItem,
    pub status: HeldStatus,
    pub by: Option<String>,
}

fn pending(e: &FeedEntry) -> bool {
    matches!(e, FeedEntry::Held(h) if h.status == HeldStatus::Pending)
}

fn is(key: &str) -> impl Fn(&FeedEntry) -> bool + '_ {
    move |e| matches!(e, FeedEntry::Held(h) if h.key == key)
}

/// A held message. One already in the feed is updated, not added twice.
pub fn held(app: &AppHandle, account: &str, channel_id: &str, channel: &str, item: &HeldItem) {
    let entry = HeldEntry {
        key: item.message_id.clone(),
        ts: item.ts,
        account: account.to_owned(),
        channel_id: channel_id.to_owned(),
        channel: channel.to_owned(),
        item: item.clone(),
        status: HeldStatus::Pending,
        by: None,
    };
    let fresh = entry.clone();
    if !update(app, FeedKind::Automod, is(&item.message_id), |e| {
        *e = FeedEntry::Held(fresh)
    }) {
        push(app, FeedKind::Automod, FeedEntry::Held(entry), pending);
    }
}

/// A mod decided, or Twitch let it go. An unknown message: nothing.
pub fn done(app: &AppHandle, message_id: &str, status: &str, by: Option<&str>) {
    let status = status_of(status);
    update(app, FeedKind::Automod, is(message_id), |e| {
        if let FeedEntry::Held(h) = e {
            h.status = status;
            h.by = by.map(str::to_owned);
        }
    });
}

/// A1 Pending rows of pairs no longer moderated go: (account, channel id).
pub fn keep_only(app: &AppHandle, pairs: &HashSet<(String, String)>) {
    remove(app, FeedKind::Automod, |e| {
        matches!(e, FeedEntry::Held(h) if h.status == HeldStatus::Pending
            && !pairs.contains(&(h.account.clone(), h.channel_id.clone())))
    });
}

fn status_of(s: &str) -> HeldStatus {
    match s.to_ascii_lowercase().as_str() {
        "approved" | "allowed" => HeldStatus::Allowed,
        "denied" => HeldStatus::Denied,
        _ => HeldStatus::Expired,
    }
}

#[cfg(test)]
mod tests {
    use super::super::ring::Ring;
    use super::*;

    fn entry(key: &str) -> FeedEntry {
        FeedEntry::Held(HeldEntry {
            key: key.into(),
            ts: 1,
            account: "a".into(),
            channel_id: "c".into(),
            channel: "chan".into(),
            item: HeldItem {
                message_id: key.into(),
                user_id: "3".into(),
                login: "x".into(),
                display: "X".into(),
                text: "hi".into(),
                reason: "AutoMod".into(),
                ts: 1,
            },
            status: HeldStatus::Pending,
            by: None,
        })
    }

    #[test]
    fn statuses_ignore_case() {
        assert_eq!(status_of("APPROVED"), HeldStatus::Allowed);
        assert_eq!(status_of("Allowed"), HeldStatus::Allowed);
        assert_eq!(status_of("DENIED"), HeldStatus::Denied);
        assert_eq!(status_of("expired"), HeldStatus::Expired);
    }

    // The ring steps `held` and `done` take, without the app.
    #[test]
    fn held_then_done_is_one_row() {
        let mut r = Ring::new(super::super::HELD);
        r.push(entry("m1"), pending);
        // Held twice: updated, not added.
        assert!(r.update(is("m1"), |e| *e = entry("m1")).is_some());
        let status = status_of("DENIED");
        r.update(is("m1"), |e| {
            if let FeedEntry::Held(h) = e {
                h.status = status;
                h.by = Some("kuni".into());
            }
        });
        let items = r.items();
        assert_eq!(items.len(), 1);
        let FeedEntry::Held(h) = &items[0] else { panic!() };
        assert_eq!((h.status, h.by.as_deref()), (HeldStatus::Denied, Some("kuni")));
    }

    #[test]
    fn pending_rows_outlive_decided_ones() {
        let mut r = Ring::new(2);
        r.push(entry("p"), pending);
        let mut d = entry("d");
        if let FeedEntry::Held(h) = &mut d {
            h.status = HeldStatus::Allowed;
        }
        r.push(d, pending);
        r.push(entry("p2"), pending);
        let keys: Vec<String> = r.items().iter().map(|e| e.key().to_owned()).collect();
        assert_eq!(keys, ["p", "p2"]);
    }
}
