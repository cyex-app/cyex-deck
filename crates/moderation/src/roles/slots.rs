//! V107-R36 The channel's VIP slots: what the Roles Manager and the usercard show, and what
//! [`super::change`] checks before a VIP grant. V107-R42 The UI's off state and count come
//! from here (`why`, `label`), so no screen has its own rule.

use serde::Serialize;
use ts_rs::TS;

/// What the VIP slots allow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum VipState {
    /// Room for more.
    Open,
    /// Every slot taken.
    Full,
    /// No slots yet: they unlock through Twitch achievements.
    Locked,
    /// Twitch didn't say; a grant tells.
    Unknown,
}

pub const FULL: &str = "Every VIP slot is taken. Remove a VIP first.";
pub const LOCKED: &str = "VIPs unlock through Twitch achievements.";

/// The channel's VIP slots.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VipSlots {
    pub used: u32,
    #[ts(optional = nullable)]
    pub allowed: Option<u32>,
    pub state: VipState,
    /// Why a VIP can't be added now; `None` when one can (or nobody knows yet).
    #[ts(optional = nullable)]
    pub why: Option<&'static str>,
    /// "VIPs 7/10", "VIPs locked", or "VIPs 7" while the limit is unknown.
    pub label: String,
}

impl VipSlots {
    pub fn new(used: u32, allowed: Option<u32>) -> VipSlots {
        let state = match allowed {
            None => VipState::Unknown,
            Some(0) => VipState::Locked,
            Some(n) if used >= n => VipState::Full,
            Some(_) => VipState::Open,
        };
        VipSlots::with(used, allowed, state)
    }

    /// Nothing known: a channel that isn't yours, or the count couldn't be read.
    pub fn unknown() -> VipSlots {
        VipSlots::new(0, None)
    }

    fn with(used: u32, allowed: Option<u32>, state: VipState) -> VipSlots {
        let why = match state {
            VipState::Full => Some(FULL),
            VipState::Locked => Some(LOCKED),
            _ => None,
        };
        let label = match (state, allowed) {
            (VipState::Locked, _) => "VIPs locked".to_owned(),
            (_, Some(n)) => format!("VIPs {used}/{n}"),
            (_, None) => format!("VIPs {used}"),
        };
        VipSlots { used, allowed, state, why, label }
    }

    /// What a grant's refusal teaches (Twitch gave no count, or it was stale).
    pub fn learn(&self, code: &str) -> VipSlots {
        let state = match code.to_ascii_uppercase().as_str() {
            "MAX_VIPS_REACHED" => VipState::Full,
            "VIP_ACHIEVEMENT_INCOMPLETE" => VipState::Locked,
            _ => return self.clone(),
        };
        VipSlots::with(self.used, self.allowed, state)
    }

    /// After a VIP was given (`on`) or taken.
    pub fn moved(&self, on: bool) -> VipSlots {
        let used = if on { self.used.saturating_add(1) } else { self.used.saturating_sub(1) };
        match self.allowed {
            Some(_) => VipSlots::new(used, self.allowed),
            // Unknown stays unknown; a taken VIP frees a full channel's slot.
            None if !on && self.state == VipState::Full => VipSlots::with(used, None, VipState::Unknown),
            None => VipSlots::with(used, None, self.state),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v107_r36_vip_slots() {
        assert_eq!(VipSlots::new(7, Some(10)).state, VipState::Open);
        assert_eq!(VipSlots::new(10, Some(10)).why, Some(FULL));
        assert_eq!(VipSlots::new(0, Some(0)).why, Some(LOCKED));
        let unknown = VipSlots::new(3, None);
        assert_eq!(unknown.why, None);
        assert_eq!(unknown.learn("MAX_VIPS_REACHED").state, VipState::Full);
        assert_eq!(unknown.learn("vip_achievement_incomplete").why, Some(LOCKED));
        assert_eq!(unknown.learn("GRANTEE_CHAT_BANNED"), unknown);
    }

    #[test]
    fn v107_r42_labels_and_moves() {
        assert_eq!(VipSlots::new(7, Some(10)).label, "VIPs 7/10");
        assert_eq!(VipSlots::new(7, None).label, "VIPs 7");
        assert_eq!(VipSlots::new(0, Some(0)).label, "VIPs locked");
        assert_eq!(VipSlots::new(9, Some(10)).moved(true).state, VipState::Full);
        assert_eq!(VipSlots::new(10, Some(10)).moved(false).state, VipState::Open);
        assert_eq!(VipSlots::new(3, None).learn("MAX_VIPS_REACHED").moved(false).state, VipState::Unknown);
        assert_eq!(VipSlots::new(0, None).moved(false).used, 0);
    }
}
